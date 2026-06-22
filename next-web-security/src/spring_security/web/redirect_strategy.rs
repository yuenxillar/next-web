use next_web_core::http::StatusCode;
use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

pub trait RedirectStrategy: Send + Sync {
    fn send_redirect(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        url: &str,
    ) -> Result<(), BoxError>;
}

#[derive(Clone)]
pub struct DefaultRedirectStrategy {
    status_code: StatusCode,
    context_relative: bool,
}

impl Default for DefaultRedirectStrategy {
    fn default() -> Self {
        Self {
            status_code: StatusCode::FOUND,
            context_relative: false,
        }
    }
}

impl RedirectStrategy for DefaultRedirectStrategy {
    fn send_redirect(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        url: &str,
    ) -> Result<(), BoxError> {
        // let redirect_url = self.calculate_redirect_url(context_path.unwrap_or_default(), url);

        // debug!("Redirecting to {redirect_url}");

        // response.insert_header("location", &redirect_url);
        // response.set_status_code(self.status_code);

        // Ok(())

        todo!();
    }
}

impl DefaultRedirectStrategy {
    fn calculate_redirect_url(&self, context_path: &str, url: &str) -> String {
        let mut url = url.to_string();
        if !is_absolute_url(&url) {
            return if self.is_context_relative() {
                url
            } else {
                format!("{}{}", context_path, url)
            };
        } else if !self.is_context_relative() {
            return url;
        } else {
            assert!(
                url.contains(context_path),
                "The fully qualified URL does not include context path."
            );
            // Find last occurrence of "://" and skip it
            url = url
                .rfind("://")
                .map(|i| &url[i + 3..])
                .map(|s| s.to_string())
                .unwrap_or(url);

            let index = url.find(context_path).unwrap();
            // Find the context_path in the part after scheme

            url = url[index + context_path.len()..].to_string();

            // If starts with '/', remove one leading slash (but keep rest)
            if url.len() > 1 && &url[0..1] == "/" {
                url = url[1..].to_owned()
            }

            url
        }
    }

    pub fn set_status_code(&mut self, status_code: StatusCode) {
        self.status_code = status_code;
    }

    pub fn is_context_relative(&self) -> bool {
        self.context_relative
    }

    pub fn set_context_relative(&mut self, context_relative: bool) {
        self.context_relative = context_relative;
    }
}

/// Utility to check if a URL is absolute (starts with scheme like http://, https://, etc.)
fn is_absolute_url(url: &str) -> bool {
    // 必须包含 "://" 且前面是合法 scheme（简单判断：以字母开头，不含空格）
    if let Some(pos) = url.find("://") {
        let scheme = &url[..pos];
        !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphabetic() || c == '+' || c == '-' || c == '.')
    } else {
        false
    }
}
