use next_web_core::anys::any_value::AnyValue;
use next_web_core::traits::http::http_request::HttpRequest;
use next_web_core::{async_trait, traits::http::http_response::HttpResponse};
use std::sync::Arc;
use uuid::Uuid;

use crate::web::csrf::{CsrfToken, CsrfTokenRepository, DefaultCsrfToken};

/// 基于 HTTP Session 的 CSRF Token Repository
#[derive(Clone)]
pub struct HttpSessionCsrfTokenRepository {
    parameter_name: String,
    header_name: String,
    session_attribute_name: String,
}

impl HttpSessionCsrfTokenRepository {
    // 默认常量
    const DEFAULT_CSRF_PARAMETER_NAME: &'static str = "_csrf";
    const DEFAULT_CSRF_HEADER_NAME: &'static str = "X-CSRF-TOKEN";

    /// 创建新的实例，使用默认配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 CSRF Token 在请求参数中的名称
    ///
    /// # Panics
    /// 如果 parameter_name 为空字符串
    pub fn set_parameter_name(&mut self, parameter_name: String) {
        assert!(
            !parameter_name.is_empty(),
            "parameterName cannot be null or empty"
        );
        self.parameter_name = parameter_name;
    }

    /// 设置 CSRF Token 在请求头中的名称
    ///
    /// # Panics
    /// 如果 header_name 为空字符串
    pub fn set_header_name(&mut self, header_name: String) {
        assert!(
            !header_name.is_empty(),
            "headerName cannot be null or empty"
        );
        self.header_name = header_name;
    }

    /// 设置 CSRF Token 在 Session 中的属性名称
    ///
    /// # Panics
    /// 如果 session_attribute_name 为空字符串
    pub fn set_session_attribute_name(&mut self, session_attribute_name: String) {
        assert!(
            !session_attribute_name.is_empty(),
            "sessionAttributeName cannot be null or empty"
        );
        self.session_attribute_name = session_attribute_name;
    }

    /// 创建新的 CSRF Token
    fn create_new_token() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn parameter_name(&self) -> &str {
        &self.parameter_name
    }

    pub fn header_name(&self) -> &str {
        &self.header_name
    }

    pub fn session_attribute_name(&self) -> &str {
        &self.session_attribute_name
    }
}

impl Default for HttpSessionCsrfTokenRepository {
    fn default() -> Self {
        let default_attr_name = format!(
            "{}.CSRF_TOKEN",
            std::any::type_name::<HttpSessionCsrfTokenRepository>()
        );

        Self {
            parameter_name: Self::DEFAULT_CSRF_PARAMETER_NAME.to_string(),
            header_name: Self::DEFAULT_CSRF_HEADER_NAME.to_string(),
            session_attribute_name: default_attr_name,
        }
    }
}

#[async_trait]
impl CsrfTokenRepository for HttpSessionCsrfTokenRepository {
    async fn generate_token(&self, _: &mut dyn HttpRequest) -> Arc<dyn CsrfToken> {
        Arc::new(DefaultCsrfToken::new(
            self.header_name.as_str(),
            self.parameter_name.as_str(),
            Self::create_new_token(),
        ))
    }

    async fn save_token(
        &self,
        token: Option<&Arc<dyn CsrfToken>>,
        request: &mut dyn HttpRequest,
        _: &mut dyn HttpResponse,
    ) {
        match token {
            None => {
                if let Some(session) = request.session(false) {
                    session.remove_attribute(&self.session_attribute_name);
                }
            }
            Some(token) => {
                let Some(session) = request.session(true) else {
                    return;
                };
                session.set_attribute(
                    &self.session_attribute_name,
                    AnyValue::Object(Box::new(Arc::clone(token))),
                );
            }
        }
    }

    async fn load_token(&self, request: &mut dyn HttpRequest) -> Option<Arc<dyn CsrfToken>> {
        match request.session(false) {
            None => return None,
            Some(session) => session
                .get_attribute(&self.session_attribute_name)
                .map(|value| value.as_object::<Arc<dyn CsrfToken>>())
                .unwrap_or_default(),
        }
    }
}
