//! The layer that runs the [`HttpFilter`]s of an application.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
    Router,
};
use next_web_context::ApplicationContextExt;
use next_web_core::{
    filter::application_filter_chain::ApplicationFilterChain,
    http::{buffer_request_body_for, BodyBufferPolicy, RequestLifecycle},
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::http_response::HttpResponse,
    },
};

use crate::{Application, ConfigurableApplicationContext};

/// The filters the layer runs, and the bodies it buffers for them.
pub(crate) struct FilterLayerState {
    /// The filters of the application, in the order they run.
    pub filters: Arc<[Arc<dyn HttpFilter>]>,
    /// The bodies the filters read, and the limit they are read with.
    pub body_buffer: BodyBufferPolicy,
}

/// Runs the filters of the application in front of the routes.
///
/// The filters run before the routes, and a filter that produced the response
/// ends the request: the session the request leaves behind is written back to
/// the session manager and reported to the client. A request that reaches the
/// routes is handed to them, which is why the lifecycle of that request is not
/// finished here: the request cannot be read after the routes consumed it.
///
/// # Arguments
///
/// * `state` - The filters and the body policy of the application.
/// * `request` - The request of the layer.
/// * `next` - The routes of the application.
pub(crate) async fn http_filter_layer<T>(
    State(state): State<Arc<FilterLayerState>>,
    mut request: Request,
    next: Next,
) -> Response
where
    T: Application + 'static,
{
    // The filters read the body of a request synchronously, so the body of the
    // requests they read is buffered before they run.
    request = match buffer_request_body_for(request, &state.body_buffer).await {
        Ok(request) => request,
        Err(error) => {
            tracing::warn!("the request is refused: {error}");
            let mut response = T::to_error_response(error.to_string());
            *response.status_mut() = StatusCode::PAYLOAD_TOO_LARGE;

            return response;
        }
    };

    // The request was prepared by the layer that prepares every request of the
    // application, and this layer is inside it, so the lifecycle does not read
    // the state of the client a second time. It does remember the session the
    // client presented, which is what `end` compares the session it leaves
    // behind with.
    let lifecycle = RequestLifecycle::begin(&mut request);
    let filter_chain = ApplicationFilterChain::new(state.filters.clone());
    let mut response = Response::new(Body::empty());

    match filter_chain.do_filter(&mut request, &mut response).await {
        // The filter already produced the response, so the routes are skipped.
        Ok(()) if response.is_committed() => {
            lifecycle.end(&mut request, &mut response);

            response
        }
        Ok(()) => next.run(request).await,
        Err(error) => {
            let mut response = T::to_error_response(error.to_string());
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            lifecycle.end(&mut request, &mut response);

            response
        }
    }
}

/// Installs the layer that runs the [`HttpFilter`]s of the application.
///
/// The filters are resolved from the application context once, when the router
/// is built, and are run by an [`ApplicationFilterChain`] in front of the
/// routes. The router is returned unchanged when the application does not
/// register a filter, so an application that does not use filters pays nothing
/// for the layer.
///
/// The pool of filters that has to run before every other filter is read from
/// the application context when the optional web security support contributed
/// it, and is skipped otherwise, so this layer also works for an application
/// that only registers filters of its own.
///
/// # Arguments
///
/// * `ctx` - The context the filters and the body policy are resolved from.
/// * `router` - The router the layer is installed on.
pub(crate) fn add_filter_layer<T>(
    ctx: &mut dyn ConfigurableApplicationContext,
    router: Router,
) -> Router
where
    T: Application + 'static,
{
    // Security filters (authn/authz, CSRF, CORS, etc.) are pushed first so they
    // run before regular business filters in the middleware chain. This ensures
    // requests are rejected as early as possible and later filters can rely on
    // an already-established security context.
    let mut http_filters = Vec::new();

    #[cfg(feature = "enable-web-security")]
    if let Some(security_filters) = ctx
        .get_singleton_option_with_name::<Arc<[Arc<dyn HttpFilter>]>>(
            next_web_context::SECURITY_FILTERS_SINGLETON_NAME,
        )
    {
        http_filters.extend(security_filters.iter().cloned());
    }

    http_filters.extend(ctx.resolve_by_type::<Arc<dyn HttpFilter>>());
    if http_filters.is_empty() {
        return router;
    }

    let body_buffer = ctx
        .resolve_by_type::<BodyBufferPolicy>()
        .into_iter()
        .next()
        .unwrap_or_default();
    let state = Arc::new(FilterLayerState {
        filters: Arc::from(http_filters),
        body_buffer,
    });

    router.layer(axum::middleware::from_fn_with_state(
        state,
        http_filter_layer::<T>,
    ))
}
