use crate::{
    error::BoxError,
    traits::http::{
        http_request::HttpRequest, http_response::HttpResponse,
        request_dispatcher::RequestDispatcher,
    },
};

pub struct DefaultRequestDispatcher {}

impl RequestDispatcher for DefaultRequestDispatcher {
    fn forward(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        Ok(())
    }

    fn include(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        Ok(())
    }
}

impl DefaultRequestDispatcher {
    pub fn handle<P: ToString>(path: P) -> Result<Self, BoxError> {
        let path = path.to_string();

        // Validate the path argument
        if !path.starts_with('/') {
            return Err("Path must start with a forward slash".into());
        }

        // Same processing order as InputBuffer / CoyoteAdapter
        // First remove query string
        let mut uri = None;
        let mut query_string = None;

        let pos = path.find('?');
        if let Some(pos) = pos {
            uri = Some(&path[0..pos]);
            query_string = Some(&path[pos + 1..]);
        } else {
            uri = Some(&path);
        }

        // Remove path parameters
        let uri_no_params = Self::strip_path_params(uri.unwrap_or_default());

        // Then normalize
        let normalized_uri = match Self::normalize(&uri_no_params) {
            Some(uri) => uri,
            None => return Err(format!("Invalid path: {}", path).into()),
        };

        // Mapping is against the normalized uri

        // Decode
        let decoded_uri = urlencoding::decode(&normalized_uri).map_err(Into::<BoxError>::into)?;
        // Security check to catch attempts to encode /../ sequences
        let _normalized_uri = Self::normalize(&decoded_uri);

        if !(decoded_uri == _normalized_uri.unwrap_or_default()) {
            return Err(format!("Invalid path: {}", path).into());
        }

        // URI needs to include the context path
        let uri = urlencoding::encode("") + uri.unwrap_or_default();

        // Use the thread local URI and mapping data
        // TODO

        Ok(DefaultRequestDispatcher {})
    }

    fn strip_path_params(path: &str) -> String {
        if !path.contains(';') {
            return path.to_string();
        }

        let mut result = String::with_capacity(path.len());
        let mut chars = path.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == ';' {
                // 跳过分号后的所有字符，直到遇到斜杠或结尾
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '/' {
                        break; // 遇到斜杠，停止跳过
                    }
                    chars.next(); // 跳过当前字符
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    fn normalize(path: &str) -> Option<String> {
        if path.is_empty() {
            return Some("".to_string());
        }

        let mut normalized = path.to_string();

        // 1. 替换连续的斜杠为单个斜杠
        normalized = Self::collapse_slashes(&normalized);

        // 2. 处理路径段
        let mut result: Vec<String> = Vec::new();

        for segment in normalized.split('/') {
            if segment.is_empty() || segment == "." {
                continue;
            }

            if segment == ".." {
                if result.pop().is_none() {
                    // 尝试退到根目录之上
                    return None;
                }
            } else {
                // 基本检查，防止明显的目录遍历
                if segment.contains("..") {
                    return None;
                }
                result.push(segment.to_string());
            }
        }

        // 3. 重新构建路径
        let mut final_path = String::new();
        for segment in &result {
            final_path.push('/');
            final_path.push_str(segment);
        }

        // 4. 处理特殊边界情况
        if final_path.is_empty() {
            if normalized.starts_with('/') {
                final_path.push('/');
            }
        } else if !normalized.starts_with('/') && final_path.starts_with('/') {
            // 如果原始路径不以/开头，但结果以/开头，移除开头的/
            final_path = final_path[1..].to_string();
        }

        // 5. 处理结尾的斜杠
        if normalized.ends_with('/') && !final_path.ends_with('/') && !final_path.is_empty() {
            final_path.push('/');
        }

        Some(final_path)
    }

    fn collapse_slashes(path: &str) -> String {
        let mut result = String::with_capacity(path.len());
        let mut last_was_slash = false;

        for ch in path.chars() {
            if ch == '/' {
                if !last_was_slash {
                    result.push(ch);
                    last_was_slash = true;
                }
            } else {
                result.push(ch);
                last_was_slash = false;
            }
        }

        result
    }
}
