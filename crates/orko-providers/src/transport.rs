//! [`Transport`] sends a POST with headers and returns the body as a stream of bytes.

use futures::Stream;
use orko_core::Result;
use std::future::Future;
use std::pin::Pin;

/// A streaming response body.
pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Vec<u8>>> + Send>>;

/// An HTTP response with a streaming body.
pub struct HttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// The body, streamed as it arrives.
    pub body: ByteStream,
}

/// The HTTP layer under [`OpenAiCompatible`](crate::OpenAiCompatible). It is
/// object safe and uses only std and `futures` types.
///
/// Implement it to use your own HTTP client, for example to add a proxy,
/// custom TLS or connection pooling.
pub trait Transport: Send + Sync {
    /// POSTs `body` to `url` with `headers` and resolves when the response
    /// head arrives. A non-2xx status comes back as a normal response; the
    /// caller decides what to do with it.
    fn post<'a>(
        &'a self,
        url: &'a str,
        headers: &'a [(String, String)],
        body: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse>> + Send + 'a>>;
}

#[cfg(feature = "transport-hyper")]
pub use hyper_impl::HyperTransport;

#[cfg(feature = "transport-hyper")]
mod hyper_impl {
    use super::{HttpResponse, Transport};
    use async_net::TcpStream;
    use bytes::Bytes;
    use futures::future::{self, Either};
    use futures::io::{AsyncRead, AsyncWrite};
    use futures::{ready, Stream};
    use futures_rustls::TlsConnector;
    use http_body_util::Full;
    use hyper::body::{Body, Incoming};
    use orko_core::{Error, Result};
    use rustls::pki_types::ServerName;
    use std::future::Future;
    use std::io;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use std::time::Duration;

    fn err(e: impl std::fmt::Display) -> Error {
        Error::Provider(format!("transport: {e}"))
    }

    /// The default transport: hyper 1.x HTTP/1.1 over `async-net` TCP, with
    /// rustls and the platform's root certificates for HTTPS.
    ///
    /// It works under any executor (Tokio, smol, `futures::executor::block_on`)
    /// because `async-net` runs its own reactor. It spawns no tasks; reading
    /// the response body is what moves the connection forward.
    ///
    /// Requests time out after [`HyperTransport::DEFAULT_TIMEOUT`]. Use
    /// [`HyperTransport::with_timeout`] to change it.
    #[derive(Clone)]
    pub struct HyperTransport {
        tls: TlsConnector,
        timeout: Duration,
    }

    impl HyperTransport {
        /// The default request timeout, 60 seconds.
        pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

        /// Creates the transport and loads the platform's root certificates.
        ///
        /// # Errors
        ///
        /// [`Error::Config`] if the platform has no root certificates we can load.
        pub fn new() -> Result<Self> {
            let mut roots = rustls::RootCertStore::empty();
            let (added, _) =
                roots.add_parsable_certificates(rustls_native_certs::load_native_certs().certs);
            if added == 0 {
                return Err(Error::Config(
                    "no native TLS root certificates found".into(),
                ));
            }
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let config = rustls::ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .map_err(|e| Error::Config(format!("TLS config: {e}")))?
                .with_root_certificates(roots)
                .with_no_client_auth();
            Ok(Self {
                tls: TlsConnector::from(Arc::new(config)),
                timeout: Self::DEFAULT_TIMEOUT,
            })
        }

        /// Sets the request timeout. It covers connecting, the TLS handshake
        /// and waiting for the response head. A long streamed body is not cut off.
        pub fn with_timeout(mut self, timeout: Duration) -> Self {
            self.timeout = timeout;
            self
        }

        async fn send(
            &self,
            url: &str,
            headers: &[(String, String)],
            body: Vec<u8>,
        ) -> Result<HttpResponse> {
            let uri: http::Uri = url.parse().map_err(err)?;
            let https = match uri.scheme_str() {
                Some("https") => true,
                Some("http") => false,
                _ => return Err(err("url scheme must be http or https")),
            };
            let authority = uri.authority().ok_or_else(|| err("url has no host"))?;
            let host = authority.host().trim_matches(|c| c == '[' || c == ']');
            let port = authority.port_u16().unwrap_or(if https { 443 } else { 80 });
            let path = uri.path_and_query().map_or("/", |p| p.as_str());

            let addrs = async_net::resolve((host, port)).await.map_err(err)?;
            if addrs.is_empty() {
                return Err(err(format_args!("`{host}` resolved to no addresses")));
            }
            // ponytail: connects to every address at once and keeps the first
            // that answers, so a dead IPv6 route doesn't hang the request.
            // RFC 8305 staggers the attempts; do that if the extra
            // connections ever become a problem.
            let attempts = addrs.into_iter().map(|a| Box::pin(TcpStream::connect(a)));
            let (tcp, _) = future::select_ok(attempts).await.map_err(err)?;
            if https {
                let name = ServerName::try_from(host.to_owned()).map_err(err)?;
                let tls = self.tls.connect(name, tcp).await.map_err(err)?;
                exchange(Io(tls), authority.as_str(), path, headers, body).await
            } else {
                exchange(Io(tcp), authority.as_str(), path, headers, body).await
            }
        }
    }

    impl Transport for HyperTransport {
        fn post<'a>(
            &'a self,
            url: &'a str,
            headers: &'a [(String, String)],
            body: Vec<u8>,
        ) -> Pin<Box<dyn Future<Output = Result<HttpResponse>> + Send + 'a>> {
            Box::pin(async move {
                let send = std::pin::pin!(self.send(url, headers, body));
                match future::select(send, async_io::Timer::after(self.timeout)).await {
                    Either::Left((resp, _)) => resp,
                    Either::Right(_) => Err(err(format_args!(
                        "request timed out after {:?}",
                        self.timeout
                    ))),
                }
            })
        }
    }

    async fn exchange<T>(
        io: T,
        host: &str,
        path: &str,
        headers: &[(String, String)],
        body: Vec<u8>,
    ) -> Result<HttpResponse>
    where
        T: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
    {
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
            .await
            .map_err(err)?;

        let mut req = http::Request::post(path).header(http::header::HOST, host);
        for (k, v) in headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let req = req.body(Full::new(Bytes::from(body))).map_err(err)?;

        // Poll the connection and the request together instead of spawning a task.
        let mut conn = Box::pin(conn);
        let send = std::pin::pin!(sender.send_request(req));
        let (resp, conn) = match future::select(send, conn.as_mut()).await {
            Either::Left((resp, _)) => (resp.map_err(err)?, Some(conn)),
            Either::Right((done, send)) => {
                done.map_err(err)?;
                (send.await.map_err(err)?, None)
            }
        };
        drop(sender); // lets the connection close once the body is read

        let status = resp.status().as_u16();
        let body = Driven {
            conn,
            body: resp.into_body(),
        };
        Ok(HttpResponse {
            status,
            body: Box::pin(body),
        })
    }

    /// Wraps a `futures::io` stream so hyper can read and write it.
    struct Io<T>(T);

    impl<T: AsyncRead + Unpin> hyper::rt::Read for Io<T> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            mut buf: hyper::rt::ReadBufCursor<'_>,
        ) -> Poll<io::Result<()>> {
            // ponytail: reads into a stack buffer and copies it over, because
            // writing into hyper's buffer directly needs unsafe. The copy is
            // cheap compared with the network read.
            let mut tmp = [0u8; 8192];
            let len = buf.remaining().min(tmp.len());
            let n = ready!(Pin::new(&mut self.0).poll_read(cx, &mut tmp[..len]))?;
            buf.put_slice(&tmp[..n]);
            Poll::Ready(Ok(()))
        }
    }

    impl<T: AsyncWrite + Unpin> hyper::rt::Write for Io<T> {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            Pin::new(&mut self.0).poll_write(cx, buf)
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.0).poll_flush(cx)
        }

        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.0).poll_close(cx)
        }
    }

    /// The response body. Each read also polls the connection, so the
    /// socket keeps working without a spawned task.
    struct Driven<C> {
        conn: Option<Pin<Box<C>>>,
        body: Incoming,
    }
    impl<C: Future<Output = hyper::Result<()>>> Stream for Driven<C> {
        type Item = Result<Vec<u8>>;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let this = &mut *self;
            if let Some(conn) = &mut this.conn {
                if let Poll::Ready(res) = conn.as_mut().poll(cx) {
                    this.conn = None;
                    res.map_err(err)?;
                }
            }
            loop {
                match Pin::new(&mut this.body).poll_frame(cx) {
                    Poll::Ready(Some(Ok(frame))) => {
                        if let Ok(data) = frame.into_data() {
                            return Poll::Ready(Some(Ok(data.to_vec())));
                        } // trailers: skip
                    }
                    Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(err(e)))),
                    Poll::Ready(None) => return Poll::Ready(None),
                    Poll::Pending => return Poll::Pending,
                }
            }
        }
    }
}
