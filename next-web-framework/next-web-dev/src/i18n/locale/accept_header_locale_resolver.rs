use next_web_core::{interface::locale_resolver::LocaleResolver, util::locale::Locale};

#[derive(Clone)]
pub struct AcceptHeaderLocaleResolver;

impl LocaleResolver for AcceptHeaderLocaleResolver {
    fn resolve_locale(
        &self,
        req: &axum::http::Request<axum::body::Body>,
    ) -> next_web_core::util::locale::Locale {
        req.headers()
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| Locale::from_language(value))
            .unwrap_or(next_web_core::util::locale::Locale::EnUs)
    }
}