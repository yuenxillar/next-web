use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::{
    core::http_security::MatchType,
    permission::manager::user_authorization_manager::UserAuthenticationManager,
};

pub(crate) async fn request_auth_middleware(
    State(user_auth_manager): State<UserAuthenticationManager>,
    req_header: HeaderMap,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    
    println!("request_auth_middleware: req_header: {:?}, req: {:?}", req_header, req);
   
    let auth_service = user_auth_manager.authentication_service();
    let router = user_auth_manager.router();
    let http_security = user_auth_manager.http_security();

    if http_security.match_type != MatchType::NotMatch {
        let login_type = auth_service.login_type(&req_header);
        let user_id = auth_service.user_id(&req_header);
        let path = req.uri().path();

        println!("path: {:?}", path);
        let permission_group = router.at(path).ok();

        let resp = Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body("Unauthorized".into())
            .unwrap();
        if let Some(group) = permission_group {
            if !user_auth_manager
                .pre_authorize(&user_id, &login_type, group)
                .await
            {
                println!("Unauthorized");
                return Err(resp);
            }
        } else {
            if http_security.match_type == MatchType::AllMatch {
                return Err(resp);
            }
        }
    }

    Ok(next.run(req).await)
}
