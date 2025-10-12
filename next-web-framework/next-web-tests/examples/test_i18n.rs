use next_web_dev::extract::Path;
use next_web_dev::i18n::locale::accept_header_locale_resolver::AcceptHeaderLocaleResolver;
use next_web_dev::extract::find_singleton::FindSingleton;
use next_web_dev::response::IntoResponse;
use next_web_dev::service::message_source_service::MessageSourceService;
use next_web_dev::traits::locale_resolver::LocaleResolver;
use next_web_dev::{
    application::Application, async_trait, context::properties::ApplicationProperties, GetMapping,
};

#[derive(Clone, Default)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&self, _properties: &ApplicationProperties) {}
}

#[GetMapping(path = "/message/{msg}")]
async fn req_message(
    Path(msg): Path<String>,
    FindSingleton(message_source_service): FindSingleton<MessageSourceService>,
    req: axum::http::Request<axum::body::Body>,
) -> impl IntoResponse {
    let locale_resolver = AcceptHeaderLocaleResolver {};
    if msg == "name" {
        message_source_service.message_with_args(msg.to_string(), & ["John", "180", "xxx"], locale_resolver.resolve_locale(&req))
        .unwrap_or(msg.into())
    }else {
        message_source_service.message_or_default(msg, locale_resolver.resolve_locale(&req))
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
