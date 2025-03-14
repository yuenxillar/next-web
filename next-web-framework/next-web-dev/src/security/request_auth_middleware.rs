use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
    // Extension,
};
use next_web_common::response::api_response::ApiResponse;
// use tracing::info;

use super::{authorization_service::AuthorizationService, login_type::LoginType};
use crate::{
    manager::user_authorization_manager::UserAuthorizationManager, util::token_util::TokenUtil,
};

async fn request_auth_middleware<T: AuthorizationService<Vec<String>> + Clone>(
    State(user_auth_manager): State<UserAuthorizationManager<T>>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiResponse<String>> {
    let login_type = LoginType::default();
    // do something with `state` and `request`...
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or(ApiResponse::fail(
            "Authorization header not found".into(),
        ))
        .map(|token| token.replace("Bearer ", ""))?;

    // local (check if the token is valid)
    let user_info = TokenUtil::decode(&token).map_err(|e| ApiResponse::fail(e.to_string()))?;

    // service (check if the token is valid)
    if !user_auth_manager.verify_token(&token, &login_type).await {
        return Err(ApiResponse::fail("Invalid token".into()));
    }

    if let Some(auth_group) = user_auth_manager.get_permission(req.method(), req.uri().path()) {
        // check if the user is authorized to access the resource
        if !user_auth_manager
            .pre_authorize(user_info.user_id(), auth_group, &login_type)
            .await
        {
            return Err(ApiResponse::fail("Unauthorized access".into()));
        }
    }
    // set the user info in the request
    req.extensions_mut().insert(user_info);

    // check if the user is authorized to access the resource
    Ok(next.run(req).await)
}
