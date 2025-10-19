
pub mod access;
pub mod authentication;
pub mod authorization;
pub mod config;
pub mod core;
pub mod crypto;
pub mod permission;
pub mod use_router;
pub mod web;



use core::filter::Filter;

type Result<R> = std::result::Result<R, R>;
pub async fn web_security_middleware(
    axum::extract::State(proxy): axum::extract::State<
        std::sync::Arc<crate::web::filter_chain_proxy::FilterChainProxy>,
    >,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response> {
    let mut resp = axum::response::Response::default();
    match proxy.do_filter(&mut req, &mut resp) {
        Ok(_) => {}
        Err(_error) => return Err(resp),
    };

    Ok(next.run(req).await)
}
