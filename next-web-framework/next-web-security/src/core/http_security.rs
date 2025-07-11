use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::permission::models::permission_group::PermissionGroup;

type BoxError = Box<dyn std::error::Error>;
type ErrorHandler = Box<dyn Fn(BoxError) -> Response + Send + Sync>;


pub struct HttpSecurity {
    pub(crate) any_match: Vec<(&'static str, PermissionGroup)>,
    pub(crate) not_match: Vec<&'static str>,
    pub(crate) match_type: MatchType,
    pub(crate) error_handler: ErrorHandler,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MatchType {
    #[default]
    AllMatch,
    OnlyMatchOwner,
    NotMatch,
}

impl HttpSecurity {
    pub fn new() -> Self {
        Self {
            any_match: Vec::new(),
            match_type: MatchType::default(),
            not_match: Vec::new(),
            error_handler: Box::new(|_| (StatusCode::UNAUTHORIZED, "Unauthorized").into_response())
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

    pub fn not_matches<P>(mut self, paths: P) -> Self
    where
        P: IntoIterator<Item = &'static str>,
    {
        for path in paths {
            self.not_match.push(path);
        }
        self
    }

    pub fn map_error<F>(mut self, f: F) -> Self
    where
        F: Fn(BoxError) -> Response + Send + Sync,
        F: 'static,
    {
        self.error_handler = Box::new(f);
        self
    }

    pub fn disable(mut self) -> Self {
        self.match_type = MatchType::OnlyMatchOwner;
        self
    }

    pub fn disable_all(mut self) -> Self {
        self.match_type = MatchType::NotMatch;
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
            match_type: self.match_type.clone(),
            error_handler: Box::new(|_| (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()),
        }
    }
}

impl Default for HttpSecurity {
    fn default() -> Self {
        Self {
            any_match: Default::default(),
            not_match: Default::default(),
            match_type: MatchType::NotMatch,
            error_handler: Box::new(|_| (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()),
        }
    }
}
