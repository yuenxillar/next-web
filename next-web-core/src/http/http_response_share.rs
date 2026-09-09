use crate::{http::Cookie, traits::http::http_response::HttpResponse};

use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Version, header::LOCATION};

pub struct HttpResponseShare {
    version: Version,
    status_code: StatusCode,
    headers: HeaderMap<HeaderValue>,
    body: Vec<u8>,
    committed: bool,
}

impl HttpResponse for HttpResponseShare {
    fn version(&self) -> Version {
        self.version
    }

    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn set_status_code(&mut self, status_code: StatusCode) {
        self.status_code = status_code;
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }

    fn headers(&self, name: &str) -> Option<Vec<&str>> {
        let values: Vec<&str> = self
            .headers
            .get_all(name)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect();
        if values.is_empty() {
            None
        } else {
            Some(values)
        }
    }

    fn append_header(&mut self, name: &str, value: &str) -> bool {
        let value = match value.parse() {
            Ok(value) => value,
            Err(_) => return false,
        };
        HeaderName::from_bytes(name.as_bytes())
            .map(|name| self.headers.append(name, value))
            .unwrap_or_default()
    }

    fn contains_header(&self, name: &str) -> bool {
        self.headers.contains_key(name)
    }

    fn insert_header(&mut self, name: &str, value: &str) -> Option<String> {
        let name = match HeaderName::from_bytes(name.as_bytes()) {
            Ok(name) => name,
            Err(_) => return None,
        };

        value.parse().ok().map(|value| {
            self.headers
                .insert(name, value)
                .and_then(|previous| previous.to_str().ok().map(ToString::to_string))
                .unwrap_or_default()
        })
    }

    fn remove_header(&mut self, name: &str) -> Option<String> {
        HeaderName::from_bytes(name.as_bytes())
            .ok()
            .and_then(|name| self.headers.remove(name))
            .and_then(|value| value.to_str().ok().map(ToString::to_string))
    }

    fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
    }

    fn set_redirect(&mut self, url: &str) {
        if let Ok(url) = HeaderValue::from_str(url) {
            self.status_code = StatusCode::SEE_OTHER;
            self.headers.insert(LOCATION, url);
        }
    }

    fn add_cookie(&mut self, cookie: Cookie) {
        let _ = self.append_header("set-cookie", &cookie.to_string());
    }

    fn is_committed(&self) -> bool {
        self.committed
    }

    fn commit(&mut self) {
        self.committed = true;
    }

    fn shared(&mut self) -> &HttpResponseShare {
        self
    }
}

impl Clone for HttpResponseShare {
    fn clone(&self) -> Self {
        Self {
            version: self.version,
            status_code: self.status_code,
            headers: self.headers.clone(),
            body: self.body.clone(),
            committed: self.committed,
        }
    }
}

impl From<&dyn HttpResponse> for HttpResponseShare {
    fn from(value: &dyn HttpResponse) -> Self {
        Self {
            version: value.version(),
            status_code: value.status_code(),
            headers: HeaderMap::new(),
            body: Vec::new(),
            committed: value.is_committed(),
        }
    }
}
