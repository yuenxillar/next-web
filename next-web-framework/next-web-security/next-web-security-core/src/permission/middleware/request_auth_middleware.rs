use axum::{
    extract::{Request, State},
    http::{HeaderMap, Response, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response}, Extension,
};

use crate::permission::{
    manager::user_authorization_manager::UserAuthenticationManager,
    models::permission_group::PermissionGroup,
};

pub(crate) async fn request_auth_middleware<R = axum::response::Response>(
    State(user_auth_manager): State<UserAuthenticationManager>,
    Extension(var): Extension<Option<String>>,
    req_header: HeaderMap,
    req: Request,
    next: Next,
) -> Result<R, R> {
    let auth_service = user_auth_manager.authentication_service();

    let login_type = auth_service.login_type(&req_header);
    let user_id = auth_service.id(&req_header);

    if user_auth_manager
        .pre_authorize(&user_id, &login_type, &PermissionGroup::default())
        .await
    {
        return Ok(next.run(req).await);
    }

    Err(Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body("")
        .unwrap())
}
