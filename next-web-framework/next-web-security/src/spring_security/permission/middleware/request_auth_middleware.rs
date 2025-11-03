use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::{
    config::web::http_security::MatchType, permission::manager::user_authorization_manager::UserAuthenticationManager
};

pub(crate) async fn request_auth_middleware(
    State(user_auth_manager): State<UserAuthenticationManager>,
    req_header: HeaderMap,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let http_security = user_auth_manager.http_security().as_ref();
    
    // 快速路径：如果不需要认证检查，直接继续
    if http_security.match_type == MatchType::NotMatch {
        return Ok(next.run(req).await);
    }

    let path = req.uri().path();
    println!("Authentication check for path: {}", path);

    // 获取路由权限组
    let permission_group = user_auth_manager.router().at(path).ok();

    // 如果没有找到权限组且匹配模式是 AllMatch，返回未授权
    if permission_group.is_none() && http_security.match_type == MatchType::AllMatch {
        return Err(unauthorized_response());
    }

    // 如果有权限组，进行认证和授权检查
    if let Some(group) = permission_group {
        let auth_service = user_auth_manager.authentication_service();
        
        // 获取登录类型和用户ID
        let login_type = auth_service.login_type(&req_header).await;
        let user_id = auth_service.user_id(&req_header).await;

        // 进行预授权检查
        if !user_auth_manager
            .pre_authorize(&user_id, &login_type, group)
            .await
        {
            println!("Unauthorized access attempt for user: {}, path: {}", user_id, path);
            return Err(unauthorized_response());
        }
    }

    // 授权通过，继续处理请求
    Ok(next.run(req).await)
}

/// 创建统一的未授权响应
fn unauthorized_response() -> Response {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body("Unauthorized".into())
        .expect("Failed to build unauthorized response")
}