//! The layer that prepares a request before the filters and the routes of an
//! application run.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use next_web_core::traits::http::http_request::HttpRequest;

#[cfg(feature = "enable-i18n")]
use crate::web::i18n::LocaleResolver;
#[cfg(feature = "enable-i18n")]
use next_web_context::i18n::LocaleContextHolder;

/// The state of the layer that prepares a request.
///
/// The layer resolves the locale of a request with the resolver the application
/// registered, which is why the resolver is handed to it when the router is
/// built instead of being read from the request: the layer runs before every
/// layer that adds an extension to the request.
pub(crate) struct PrepareRequestState {
    /// The resolver of the locale of a request, when the i18n support is built.
    #[cfg(feature = "enable-i18n")]
    locale_resolver: Option<Arc<dyn LocaleResolver>>,
}

impl PrepareRequestState {
    /// Creates the state of the layer.
    ///
    /// # Arguments
    ///
    /// * `locale_resolver` - The resolver of the locale of a request, when the
    ///   i18n support is built.
    pub(crate) fn new(
        #[cfg(feature = "enable-i18n")] locale_resolver: Option<Arc<dyn LocaleResolver>>,
    ) -> Self {
        Self {
            #[cfg(feature = "enable-i18n")]
            locale_resolver,
        }
    }
}

/// Prepares the per-request state of the request.
///
/// The attributes, the cookies, and the session of the client are read from the
/// request, so the filters and the handlers of the application can read the
/// state a previous request of the client left behind. The layer is installed
/// outside every other layer, so the state is available to all of them.
///
/// The locale of the request is resolved once, with the resolver of the
/// application, and is associated with the request for the rest of the chain:
/// [`LocaleContextHolder`] answers it where the framework looks a message up
/// without being given a locale, and a handler reads it with
/// [`RequestLocaleHolder`](crate::web::i18n::RequestLocaleHolder).
///
/// The layer does not finish the request: the request is consumed by the
/// service it is handed to, so a caller that only prepares a request cannot read
/// it once the response was produced. The layer that runs the filters of the
/// application finishes the request, because it owns the request until the
/// response is produced.
///
/// # Arguments
///
/// * `state` - The resolver of the locale of the request.
/// * `request` - The request of the layer.
/// * `next` - The rest of the application.
pub(crate) async fn prepare_request_layer(
    State(state): State<Arc<PrepareRequestState>>,
    mut request: Request,
    next: Next,
) -> Response {
    HttpRequest::ready(&mut request);

    #[cfg(feature = "enable-i18n")]
    {
        // The resolver of the application wins, which is what makes a cookie or
        // a session resolver of an application work; without one the locale the
        // client asked for is used, which is the `Accept-Language` header.
        let locale = state
            .locale_resolver
            .as_deref()
            .map(|locale_resolver| locale_resolver.resolve_locale(&request))
            .unwrap_or_else(|| HttpRequest::locale(&request).unwrap_or_default());

        return LocaleContextHolder::scope_locale(locale, next.run(request)).await;
    }

    #[cfg(not(feature = "enable-i18n"))]
    {
        let _ = state;

        next.run(request).await
    }
}

#[cfg(all(test, feature = "enable-i18n"))]
mod tests {
    use super::*;
    use crate::web::i18n::RequestLocaleHolder;
    use axum::{Router, body::Body, routing::get};
    use next_web_context::Locale;
    use next_web_core::traits::http::http_response::HttpResponse;
    use tower::ServiceExt;

    /// A resolver that answers the locale of the test.
    struct FixedLocaleResolver(Locale);

    impl LocaleResolver for FixedLocaleResolver {
        fn resolve_locale(&self, _request: &dyn HttpRequest) -> Locale {
            self.0.clone()
        }

        fn set_locale(
            &self,
            _request: &mut dyn HttpRequest,
            _response: Option<&mut dyn HttpResponse>,
            _locale: Option<&Locale>,
        ) -> Result<(), crate::web::i18n::LocaleResolverError> {
            Err(crate::web::i18n::LocaleResolverError::unsupported(
                "the resolver of the test only resolves a locale",
            ))
        }
    }

    /// Answers the locale the request was associated with.
    async fn locale_of_the_request() -> String {
        format!(
            "{}|{}",
            RequestLocaleHolder::locale().unwrap_or_default(),
            LocaleContextHolder::locale()
        )
    }

    #[tokio::test]
    async fn the_locale_of_a_request_is_resolved_with_the_resolver_of_the_application() {
        let state = Arc::new(PrepareRequestState::new(Some(Arc::new(
            FixedLocaleResolver(Locale::new("zh", "CN")),
        ))));
        let router = Router::new()
            .route("/locale", get(locale_of_the_request))
            .layer(axum::middleware::from_fn_with_state(
                state,
                prepare_request_layer,
            ));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/locale")
                    .header("accept-language", "en-US")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("the router answers");
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .expect("the body is read");

        // The resolver of the application wins over the header of the client, and
        // the locale is visible to the request holder and to the locale context
        // holder a message source reads.
        assert_eq!(std::str::from_utf8(&body).unwrap(), "zh_CN|zh_CN");
    }
}
