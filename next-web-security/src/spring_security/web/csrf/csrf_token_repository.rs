use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::web::csrf::{
    repository_deferred_csrf_token::RepositoryDeferredCsrfToken, CsrfToken, DeferredCsrfToken,
};

#[async_trait]
pub trait CsrfTokenRepository
where
    Self: Send + Sync,
{
    /// 生成一个新的 CSRF Token
    ///
    /// # 参数
    /// * `request` - HTTP 请求对象
    ///
    /// # 返回值
    /// 返回生成的 CSRF Token
    async fn generate_token(&self, request: &mut dyn HttpRequest) -> Arc<dyn CsrfToken>;

    /// 保存 CSRF Token
    ///
    /// 如果 token 为 None，则表示删除 token
    ///
    /// # 参数
    /// * `token` - 要保存的 CSRF Token，或 None 表示删除
    /// * `request` - HTTP 请求对象
    /// * `response` - HTTP 响应对象
    async fn save_token(
        &self,
        token: Option<&Arc<dyn CsrfToken>>,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    );

    /// 从请求中加载 CSRF Token
    ///
    /// # 参数
    /// * `request` - HTTP 请求对象
    ///
    /// # 返回值
    /// 返回加载的 CSRF Token，如果不存在则返回 None
    async fn load_token(&self, request: &mut dyn HttpRequest) -> Option<Arc<dyn CsrfToken>>;
}

/// 延迟加载 CSRF Token
///
/// 返回一个 DeferredCsrfToken，它会缓存 token 以避免重复加载
///
/// # 参数
/// * `request` - HTTP 请求对象
/// * `response` - HTTP 响应对象
///
/// # 返回值
/// 返回一个 DeferredCsrfToken 实例
pub fn load_deferred_token(
    _self: Arc<dyn CsrfTokenRepository>,
    _: &mut dyn HttpRequest,
    _: &mut dyn HttpResponse,
) -> Arc<dyn DeferredCsrfToken> {
    Arc::new(RepositoryDeferredCsrfToken::new(_self))
}
