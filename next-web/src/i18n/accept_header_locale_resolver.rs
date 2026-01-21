use axum::extract::FromRequestParts;
use next_web_core::{traits::locale_resolver::LocaleResolver, util::locale::Locale};
use reqwest::StatusCode;

#[derive(Clone)]
pub struct AcceptHeaderLocaleResolver(pub Locale);

impl LocaleResolver for AcceptHeaderLocaleResolver {
    fn resolve_locale(req: &axum::extract::Request) -> next_web_core::util::locale::Locale {
        req.headers()
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| Locale::from_language(value))
            .unwrap_or(next_web_core::util::locale::Locale::EnUs)
    }
}

impl<S> FromRequestParts<S> for AcceptHeaderLocaleResolver
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let locale = parts
            .headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| Locale::from_language(value))
            .unwrap_or(next_web_core::util::locale::Locale::EnUs);

        Ok(AcceptHeaderLocaleResolver(locale))
    }
}
