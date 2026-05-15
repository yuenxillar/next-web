use axum::http::{Method, StatusCode};
use next_web_core::{
    anys::any_map::AnyMap,
    error::BoxError,
};

use crate::{
    core::{
        context::security_context_holder::SecurityContextHolder,
        filter::Filter,
    },
    web::{
        authentication::preauth::abstract_pre_authenticated_processing_filter::{
            block_on, NEXT_SECURITY_AUTHENTICATION,
        },
        redirect_strategy::{DefaultRedirectStrategy, RedirectStrategy},
    },
};

#[derive(Clone)]
pub struct LogoutFilter {
    logout_url: Box<str>,
    logout_success_url: Option<Box<str>>,
    redirect_strategy: std::sync::Arc<dyn RedirectStrategy>,
}

impl Default for LogoutFilter {
    fn default() -> Self {
        Self {
            logout_url: "/logout".into(),
            logout_success_url: Some("/login?logout".into()),
            redirect_strategy: std::sync::Arc::new(DefaultRedirectStrategy::default()),
        }
    }
}

impl LogoutFilter {
    pub fn set_logout_url(&mut self, logout_url: impl Into<Box<str>>) {
        let logout_url = logout_url.into();
        assert!(logout_url.starts_with('/'), "logout_url must start with /");
        self.logout_url = logout_url;
    }

    pub fn set_logout_success_url(&mut self, logout_success_url: &str) {
        assert!(
            logout_success_url.starts_with('/'),
            "logout_success_url must start with /"
        );
        self.logout_success_url = Some(logout_success_url.into());
    }

    fn requires_logout(&self, request: &axum::extract::Request) -> bool {
        request.method() == Method::POST && request.uri().path() == self.logout_url.as_ref()
    }
}

impl Filter for LogoutFilter {
    fn do_filter(
        &self,
        req: &mut axum::extract::Request,
        res: &mut axum::response::Response,
    ) -> Result<(), BoxError> {
        if !self.requires_logout(req) {
            return Ok(());
        }

        SecurityContextHolder::clear_context();
        if let Some(any_map) = req.extensions().get::<AnyMap>() {
            block_on(any_map.remove(NEXT_SECURITY_AUTHENTICATION));
        }

        if let Some(url) = &self.logout_success_url {
            self.redirect_strategy.send_redirect(None, url, res);
        } else {
            *res.status_mut() = StatusCode::NO_CONTENT;
        }
        Ok(())
    }
}
