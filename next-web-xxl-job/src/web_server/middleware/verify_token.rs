use std::sync::Arc;

use axum::{extract::Request, extract::State, middleware::Next, response::Response};

use crate::web_server::{models::api_result::XxlApiResult, state::XxlJobAppState};

pub(crate) async fn verify_token_middleware(
    State(state): State<Arc<XxlJobAppState>>,
    req: Request,
    next: Next,
) -> Result<Response, XxlApiResult<()>> {
    if !req
        .headers()
        .get("XXL-JOB-ACCESS-TOKEN")
        .map(|value| value.to_str().unwrap_or_default())
        .map(|token| {
            state.client_config.access_token.is_empty()
                || state.client_config.access_token.as_str() == token
        })
        .unwrap_or(false)
    {
        return Err(XxlApiResult::fail(Some(
            "access-token is error".to_string(),
        )));
    }

    Ok(next.run(req).await)
}
