use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::permission::{
    manager::user_authorization_manager::UserAuthenticationManager,
    models::permission_group::PermissionGroup,
    service::authentication_service::AuthenticationService,
};

type Result<T> = std::result::Result<T, Response>;


pub async fn request_auth_middleware<S, R>(
    State(user_auth_manager): State<UserAuthenticationManager<S>>,
    req: Request,
    next: Next,
) -> Result<Response>
where
    S: AuthenticationService,
    R: IntoResponse,
{
    let http_security = &user_auth_manager.http_security;

    if !http_security.all_match || http_security.any_match.is_empty() {
        return Ok(next.run(req).await);
    }

    let group = PermissionGroup::default();
    let auth = user_auth_manager
        .pre_authorize(req.headers(), &group)
        .await;

    if !auth {
        return Err((http_security.error_handler)("".into()));
    }

    // do something with `state` and `request`...
    // let token = req
    //     .headers()
    //     .get(header::AUTHORIZATION)
    //     .and_then(|header| header.to_str().ok())
    //     .ok_or(ApiResponse::fail("Authorization header not found.".into()))
    //     .map(|token| token.replace("Bearer ", ""))?;

    // check if the user is authorized to access the resource
    Ok(next.run(req).await)
}
