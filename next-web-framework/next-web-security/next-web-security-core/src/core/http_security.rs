use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::permission::models::permission_group::PermissionGroup;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub struct HttpSecurity {
    pub(crate) any_match: Vec<(&'static str, PermissionGroup)>,
    pub(crate) not_match: Vec<&'static str>,
    pub(crate) all_match: bool,
    pub(crate) error_handler: Box<dyn Fn(BoxError) -> Response>,
}

impl HttpSecurity {
    pub fn new() -> Self {
        Self {
            any_match: Vec::new(),
            all_match: true,
            not_match: Vec::new(),
            error_handler: Box::new(|_| (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()),
        }
    }

    pub fn any_match<F>(mut self, path: &'static str, f: F) -> Self
    where
        F: FnOnce(PermissionGroup) -> PermissionGroup,
    {
        let permission_group = f(PermissionGroup::default());
        self.any_match.push((path, permission_group));
        self
    }

    pub fn not_match(mut self, path: &'static str) -> Self {
        self.not_match.push(path);
        self
    }

    pub fn map_error<F>(mut self, f: F) -> Self
    where
        F: Fn(BoxError) -> Response,
        F: 'static,
    {
        self.error_handler = Box::new(f);
        self
    }

    pub fn disable(mut self) -> Self {
        self.all_match = false;
        self
    }

    pub fn disable_all(mut self) -> Self {
        self.all_match = false;
        self.any_match.clear();
        self.not_match.clear();
        self
    }
}

impl Clone for HttpSecurity {
    fn clone(&self) -> Self {
        Self {
            any_match: self.any_match.clone(),
            not_match: self.not_match.clone(),
            all_match: self.all_match,
            error_handler: Box::new(|_| (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()),
        }
    }
}