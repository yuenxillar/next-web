use axum::http::StatusCode;
use next_web_core::traits::http::http_request::HttpRequest;
use next_web_core::traits::http::http_response::HttpResponse;
use std::collections::HashMap;
use std::fmt;
use urlencoding::encode;

/// 重定向视图 - 重定向到绝对、上下文相对或当前请求相对的 URL
/// 将所有模型属性暴露为 HTTP 查询参数
#[derive(Debug, Clone)]
pub struct RedirectView {
    url: String,
    context_relative: bool,
    http_10_compatible: bool,
    encoding_scheme: String,
}

impl RedirectView {
    /// 默认编码方案: UTF-8
    pub const DEFAULT_ENCODING_SCHEME: &'static str = "UTF-8";

    pub fn new() -> Self {
        Self {
            url: String::new(),
            context_relative: false,
            http_10_compatible: true,
            encoding_scheme: Self::DEFAULT_ENCODING_SCHEME.to_string(),
        }
    }

    /// 使用给定的 URL 创建新的 RedirectView
    /// 给定的 URL 将被视为相对于 Web 服务器，而不是相对于当前 ServletContext
    pub fn with_url(url: &str) -> Self {
        Self {
            url: url.to_string(),
            context_relative: false,
            http_10_compatible: true,
            encoding_scheme: Self::DEFAULT_ENCODING_SCHEME.to_string(),
        }
    }

    /// 使用给定的 URL 创建新的 RedirectView
    pub fn with_url_and_context(url: &str, context_relative: bool) -> Self {
        Self {
            url: url.to_string(),
            context_relative,
            http_10_compatible: true,
            encoding_scheme: Self::DEFAULT_ENCODING_SCHEME.to_string(),
        }
    }

    /// 使用给定的 URL 创建新的 RedirectView
    pub fn with_url_context_and_http(
        url: &str,
        context_relative: bool,
        http_10_compatible: bool,
    ) -> Self {
        Self {
            url: url.to_string(),
            context_relative,
            http_10_compatible,
            encoding_scheme: Self::DEFAULT_ENCODING_SCHEME.to_string(),
        }
    }

    // Getters
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn context_relative(&self) -> bool {
        self.context_relative
    }

    pub fn http_10_compatible(&self) -> bool {
        self.http_10_compatible
    }

    pub fn encoding_scheme(&self) -> &str {
        &self.encoding_scheme
    }

    // Setters
    pub fn set_url(&mut self, url: &str) {
        self.url = url.to_string();
    }

    /// 设置是否将以斜杠("/")开头的给定 URL 解释为相对于当前上下文
    pub fn set_context_relative(&mut self, context_relative: bool) {
        self.context_relative = context_relative;
    }

    /// 设置是否与 HTTP 1.0 客户端保持兼容
    pub fn set_http_10_compatible(&mut self, http_10_compatible: bool) {
        self.http_10_compatible = http_10_compatible;
    }

    /// 设置此视图的编码方案。默认为 UTF-8
    pub fn set_encoding_scheme(&mut self, encoding_scheme: &str) {
        self.encoding_scheme = encoding_scheme.to_string();
    }

    /// 将模型转换为请求参数并重定向到给定的 URL
    pub fn render_merged_output_model(
        &self,
        model: Option<HashMap<String, String>>,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) {
        // Prepare name URL.
        let mut target_url = String::new();

        if self.context_relative && self.url.starts_with('/') {
            // Do not apply context path to relative URLs.
            target_url.push_str(request.context_path().unwrap_or_default());
        }

        target_url.push_str(&self.url);

        // 附加查询属性
        let target_url = self.append_query_properties(target_url, model);

        // 发送重定向
        self.send_redirect(&target_url, response);
    }

    /// 将查询属性附加到重定向 URL
    fn append_query_properties(
        &self,
        mut target_url: String,
        model: Option<HashMap<String, String>>,
    ) -> String {
        // 提取锚点片段（如果有）
        let fragment = if let Some(anchor_index) = target_url.find('#') {
            let fragment = target_url[anchor_index..].to_string();
            target_url = target_url[..anchor_index].to_string();
            Some(fragment)
        } else {
            None
        };

        // 获取查询属性
        let query_props = self.query_properties(model.unwrap_or_default());

        if !query_props.is_empty() {
            // 如果还没有参数，我们需要一个 "?"
            let separator = if target_url.contains('?') { '&' } else { '?' };
            target_url.push(separator);

            // 构建查询字符串
            let query_string: Vec<String> = query_props
                .iter()
                .map(|(key, value)| {
                    let encoded_key = self.url_encode(key);
                    let encoded_value = self.url_encode(value);
                    format!("{}={}", encoded_key, encoded_value)
                })
                .collect();

            target_url.push_str(&query_string.join("&"));
        }

        // 附加锚点片段（如果有）到 URL 末尾
        if let Some(fragment) = fragment {
            target_url.push_str(&fragment);
        }

        target_url
    }

    /// URL 编码给定的输入字符串
    fn url_encode(&self, input: &str) -> String {
        encode(input).into_owned()
    }

    /// 确定查询字符串的键值对
    fn query_properties(&self, model: HashMap<String, String>) -> HashMap<String, String> {
        model
    }

    /// 发送重定向回 HTTP 客户端
    fn send_redirect(&self, target_url: &str, response: &mut dyn HttpResponse) {
        // 在 Rust 中，我们假设 URL 已经正确编码
        let encoded_redirect_url = target_url.to_string();

        if self.http_10_compatible {
            // Always send status code 302.
            response.set_status_code(StatusCode::FOUND);
            response.insert_header("location".as_bytes(), &encoded_redirect_url);
        } else {
            response.set_redirect(&encoded_redirect_url);
        }
    }
}

impl Default for RedirectView {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RedirectView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RedirectView{{url: {}, context_relative: {}, http_10_compatible: {}}}",
            self.url, self.context_relative, self.http_10_compatible
        )
    }
}
