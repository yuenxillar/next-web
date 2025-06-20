use axum::{
    Extension,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::permission::{
    manager::user_authorization_manager::UserAuthenticationManager,
    models::permission_group::PermissionGroup,
};

pub(crate) async fn request_auth_middleware(
    State(user_auth_manager): State<UserAuthenticationManager>,
    req_header: HeaderMap,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
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
        .body("".into())
        .unwrap())
}
