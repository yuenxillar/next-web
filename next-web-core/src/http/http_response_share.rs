use crate::{http::Cookie, traits::http::http_response::HttpResponse};

use axum::http::{HeaderName, StatusCode, Version};

pub struct HttpResponseShare {}

impl HttpResponse for HttpResponseShare {
    fn version(&self) -> Version {
        todo!()
    }

    fn status_code(&self) -> StatusCode {
        todo!()
    }

    fn set_status_code(&mut self, status_code: StatusCode) {
        todo!()
    }

    fn header(&self, name: &str) -> Option<&str> {
        todo!()
    }

    fn headers(&self, name: &str) -> Option<Vec<&str>> {
        todo!()
    }

    fn append_header(&mut self, name: &str, value: &str) -> bool {
        todo!()
    }

    fn contains_header(&self, name: &str) -> bool {
        todo!()
    }

    fn insert_header(&mut self, name: &str, value: &str) -> Option<String> {
        todo!()
    }

    fn remove_header(&mut self, name: &str) -> Option<String> {
        todo!()
    }

    fn set_body(&mut self, body: Vec<u8>) {
        todo!()
    }

    fn set_redirect(&mut self, url: &str) {
        todo!()
    }

    fn add_cookie(&mut self, cookie: Cookie) {
        todo!()
    }

    fn is_committed(&self) -> bool {
        todo!()
    }

    fn finish(&mut self) {
        todo!()
    }

    fn shared(&mut self) -> &HttpResponseShare {
        todo!()
    }
}

impl Clone for HttpResponseShare {
    fn clone(&self) -> Self {
        Self {}
    }
}

impl From<&dyn HttpResponse> for HttpResponseShare {
    fn from(value: &dyn HttpResponse) -> Self {
        HttpResponseShare {}
    }
}
