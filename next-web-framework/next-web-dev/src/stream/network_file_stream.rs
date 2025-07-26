use axum::response::Response;
use futures::FutureExt;
use next_web_core::interface::stream::into_response_stream::IntoRespnoseStream;
use once_cell::sync::Lazy;
use reqwest::Method;

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| reqwest::Client::new());

pub struct NetworkFileStream {
    pub url: String,
    pub method: String,
}

impl IntoRespnoseStream for NetworkFileStream {
    fn into_response_stream(self, target_rate: usize) -> Response {
        // let result = CLIENT.request(
        //     Method::from_bytes(self.method.as_bytes()).unwrap_or(Method::GET),
        //     self.url,
        // ).send().then(|respnose| {
        //     async {
        //         respnose.map(|body| body.bytes_stream())
        //     }
        // });

        Response::new("body".into())
    }
}
