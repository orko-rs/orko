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

#[cfg(feature = "transport-tokio")]
pub use tokio_impl::TokioTransport;

/// [`TokioTransport`]: hyper with hyper-util and hyper-rustls, on Tokio's IO.
#[cfg(feature = "transport-tokio")]
mod tokio_impl {
    use super::{HttpResponse, Transport};
    use bytes::Bytes;
    use futures::future::{self, Either};
    use futures::Stream;
    use http_body_util::Full;
    use hyper::body::{Body, Incoming};
    use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
    use hyper_util::client::legacy::connect::HttpConnector;
    use orko_core::{Error, Result};
    use std::future::Future;
    use std::pin::{pin, Pin};
    use std::task::{Context, Poll};
    use std::time::Duration;
    use tower_service::Service;

    /// The built-in transport: hyper 1.x HTTP/1.1 on Tokio's IO, with
    /// hyper-rustls and the platform's root certificates for HTTPS.
    ///
    /// It must be polled inside a Tokio runtime with IO and time enabled. It
    /// spawns no tasks; reading the response body is what moves the connection
    /// forward. hyper-util's connector races IPv4 and IPv6 (happy eyeballs), so
    /// a dead IPv6 route doesn't hang the request.
    ///
    /// Other runtimes don't need it: implement [`Transport`] for your own client
    /// and pass it to [`OpenAiCompatible::with_transport`](crate::OpenAiCompatible::with_transport).
    ///
    /// Requests time out after [`TokioTransport::DEFAULT_TIMEOUT`]. Use
    /// [`TokioTransport::with_timeout`] to change it.
    // INFO: opens a new connection for every request. If handshake latency
    // matters, pass a `Transport` that pools connections.
    #[derive(Clone)]
    pub struct TokioTransport {
        connector: HttpsConnector<HttpConnector>,
        timeout: Duration,
    }

    impl TokioTransport {
        /// The default request timeout, 60 seconds.
        pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

        /// Creates the transport and loads the platform's root certificates.
        ///
        /// # Errors
        ///
        /// [`orko_core::Error::Config`] if the platform has no root certificates
        /// we can load.
        pub fn new() -> Result<Self> {
            let connector = HttpsConnectorBuilder::new()
                .with_tls_config(tls_config()?)
                .https_or_http()
                .enable_http1()
                .build();
            Ok(Self {
                connector,
                timeout: Self::DEFAULT_TIMEOUT,
            })
        }

        /// Sets the request timeout. It covers connecting, the TLS handshake and
        /// waiting for the response head. A long streamed body is not cut off.
        // INFO: there is no timeout between body chunks, so a server that
        // stops partway through a stream still hangs. Add one in `Driven` if that
        // happens in practice.
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
            let io = self
                .connector
                .clone()
                .call(uri.clone())
                .await
                .map_err(err)?;
            exchange(io, &uri, headers, body).await
        }
    }

    impl Transport for TokioTransport {
        fn post<'a>(
            &'a self,
            url: &'a str,
            headers: &'a [(String, String)],
            body: Vec<u8>,
        ) -> Pin<Box<dyn Future<Output = Result<HttpResponse>> + Send + 'a>> {
            Box::pin(with_timeout(
                self.send(url, headers, body),
                tokio::time::sleep(self.timeout),
                self.timeout,
            ))
        }
    }

    fn err(e: impl std::fmt::Display) -> Error {
        Error::Provider(format!("transport: {e}"))
    }

    /// A rustls client config with the ring provider and the platform's root
    /// certificates.
    fn tls_config() -> Result<rustls::ClientConfig> {
        let mut roots = rustls::RootCertStore::empty();
        let (added, _) =
            roots.add_parsable_certificates(rustls_native_certs::load_native_certs().certs);
        if added == 0 {
            return Err(Error::Config(
                "no native TLS root certificates found".into(),
            ));
        }
        let provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());
        Ok(rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|e| Error::Config(format!("TLS config: {e}")))?
            .with_root_certificates(roots)
            .with_no_client_auth())
    }

    /// Runs `request`, failing if `sleep` finishes first.
    async fn with_timeout(
        request: impl Future<Output = Result<HttpResponse>>,
        sleep: impl Future,
        timeout: Duration,
    ) -> Result<HttpResponse> {
        match future::select(pin!(request), pin!(sleep)).await {
            Either::Left((resp, _)) => resp,
            Either::Right(_) => Err(err(format_args!("request timed out after {timeout:?}"))),
        }
    }

    /// Sends one POST over an open connection and returns the streaming response.
    async fn exchange<T>(
        io: T,
        uri: &http::Uri,
        headers: &[(String, String)],
        body: Vec<u8>,
    ) -> Result<HttpResponse>
    where
        T: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
    {
        let host = uri.authority().ok_or_else(|| err("url has no host"))?;
        let path = uri.path_and_query().map_or("/", |p| p.as_str());
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
            .await
            .map_err(err)?;

        let mut req = http::Request::post(path).header(http::header::HOST, host.as_str());
        for (k, v) in headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let req = req.body(Full::new(Bytes::from(body))).map_err(err)?;

        // Poll the connection and the request together instead of spawning a task.
        let mut conn = Box::pin(conn);
        let send = pin!(sender.send_request(req));
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

    /// The response body. Each read also polls the connection, so the socket
    /// keeps working without a spawned task.
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
