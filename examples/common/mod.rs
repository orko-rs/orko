//! A fake [`Transport`] the examples use when no API key is set.

use orko::providers::{HttpResponse, Transport};
use orko::Result;
use std::future::Future;
use std::pin::Pin;

/// Returns a fixed SSE response with the text `"Hello from "` followed by `.0`.
// INFO: `.0` goes into the JSON unescaped, so don't put quotes in it.
pub struct Canned(pub &'static str);

impl Transport for Canned {
    fn post<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        _body: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse>> + Send + 'a>> {
        let sse = format!(
            concat!(
                "data: {{\"choices\":[{{\"delta\":{{\"content\":\"Hello from \"}}}}]}}\n\n",
                "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{}\"}},\"finish_reason\":\"stop\"}}]}}\n\n",
                "data: [DONE]\n\n",
            ),
            self.0
        );
        Box::pin(async move {
            Ok(HttpResponse {
                status: 200,
                body: Box::pin(futures::stream::iter([Ok(sse.into_bytes())])),
            })
        })
    }
}
