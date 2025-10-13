use std::sync::Arc;

use axum::{
    extract::Request, http::StatusCode, middleware::Next, response::Response, Extension
};

/// 幂等中间件
pub(crate) async fn idempotency_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {

    // let configurer = req.extensions().get::<Arc<dyn Idempotency>>();    
    
    // let idempotency_key = req.headers()
    //     .get("Idempotency-Key")
    //     .and_then(|v| v.to_str().ok())
    //     .ok_or(StatusCode::BAD_REQUEST)?;

    
    // let exists: bool = conn.get::<_, bool>(format!("idemp:{}", idempotency_key)).await.unwrap_or(false);
    // if exists {
    //     // 返回缓存的响应或特定错误
    //     return Err(StatusCode::CONFLICT); // 或返回上次结果
    // }

    // // 标记已使用（可设置 TTL）
    // let _: () = conn.set_ex(format!("idemp:{}", idempotency_key), "1", 3600).await
    //     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // crate::configurer::http_method_handler_configurer::HttpMethodHandlerConfigurer::default();
    Ok(next.run(req).await)
}