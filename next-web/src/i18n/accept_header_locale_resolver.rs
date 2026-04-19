use axum::extract::{FromRequestParts, Request};
use next_web_core::{traits::locale_resolver::LocaleResolver, util::locale::Locale};
use axum::http::{StatusCode ,header::ACCEPT_LANGUAGE};

#[derive(Clone)]
pub struct AcceptHeaderLocaleResolver<T = Locale>(pub T);

impl LocaleResolver for AcceptHeaderLocaleResolver {
    fn resolve_locale(&self, _req: &mut Request) -> Locale {
       self.0
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
            .get(ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| Locale::from_accept_language(value))
            .unwrap_or(Locale::EnUs);

        Ok(AcceptHeaderLocaleResolver(locale))
    }
}
