#[cfg(feature = "user-friendly")]
mod shiro;
#[cfg(feature = "user-friendly")]
pub use shiro::*;

#[cfg(feature = "comprehensive")]
mod spring_security;
#[cfg(feature = "comprehensive")]
pub use spring_security::*;

// type Result<R> = std::result::Result<R, R>;
// pub async fn web_security_middleware(
//     axum::extract::State(proxy): axum::extract::State<
//         std::sync::Arc<crate::web::filter_chain_proxy::FilterChainProxy>,
//     >,
//     mut req: axum::extract::Request,
//     next: axum::middleware::Next,
// ) -> Result<axum::response::Response> {
//     let mut resp = axum::response::Response::default();
//     match proxy.do_filter(&mut req, &mut resp) {
//         Ok(_) => {}
//         Err(_error) => return Err(resp),
//     };

//     Ok(next.run(req).await)
// }
