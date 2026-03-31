use next_web::ApplicationContext;
use next_web::extract::Path;
use next_web::extract::find_singleton::FindSingleton;
use next_web::i18n::accept_header_locale_resolver::AcceptHeaderLocaleResolver;
use next_web::i18n::message_source_service::MessageSourceService;
use next_web::traits::locale_resolver::LocaleResolver;
use next_web::{
    application::Application, async_trait, context::properties::ApplicationProperties,
    macros::bind::get_mapping,
};

#[derive(Clone, Default)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();
    /// initialize the middleware.
    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) {
    }
}

#[get_mapping(path = "/message1/{msg}")]
async fn req_message1(
    Path(msg): Path<String>,
    FindSingleton(message_source_service): FindSingleton<MessageSourceService>,
    req: axum::http::Request<axum::body::Body>,
) -> impl IntoResponse {
    let locale_resolver = AcceptHeaderLocaleResolver::resolve_locale(&req);
    if msg == "name" {
        message_source_service
            .message_with_args(msg.to_string(), &["John", "180", "xxx"], locale_resolver)
            .unwrap_or(msg.into())
    } else {
        message_source_service.message_or_default(msg, locale_resolver)
    }
}

// #[translation]
#[get_mapping(path = "/message2/{msg}")]
async fn req_message2(
    Path(msg): Path<String>,
    FindSingleton(message_source_service): FindSingleton<MessageSourceService>,
    locale: AcceptHeaderLocaleResolver,
) -> impl IntoResponse {
    ""
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
