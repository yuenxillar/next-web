use std::sync::Arc;

use next_web::ApplicationContext;
use next_web::context::MessageSource;
use next_web::extract::Path;
use next_web::extract::find_singleton::FindSingleton;
use next_web::i18n::accept_header_locale_resolver::AcceptHeaderLocaleResolver;
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[get_mapping(path = "/message1/{msg}")]
async fn req_message1(
    Path(msg): Path<String>,
    #[find] FindSingleton(message_source): FindSingleton<Arc<dyn MessageSource>>,
    req: axum::http::Request<axum::body::Body>,
) -> impl IntoResponse {
    let locale = AcceptHeaderLocaleResolver::resolve_locale(&req);
    if msg == "name" {
        message_source
            .message_with_args(&msg, &["John", "180", "xxx"], locale)
            .await
            .unwrap_or(msg.into())
    } else {
        message_source.message_or_default(&msg, locale).await
    }
}


#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
