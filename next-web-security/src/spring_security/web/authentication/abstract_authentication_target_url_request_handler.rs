use axum::extract::Request;

use crate::web::redirect_strategy::{DefaultRedirectStrategy, RedirectStrategy};

pub struct AbstractAuthenticationTargetUrlRequestHandler {
    target_url_parameter: Option<Box<str>>,
    default_target_url: Box<str>,

    always_use_default_target_url: bool,
    use_referer: bool,

    redirect_strategy: DefaultRedirectStrategy,
}

impl AbstractAuthenticationTargetUrlRequestHandler {
    pub fn get_target_url_parameter(&self) -> String {
        self.target_url_parameter
            .as_deref()
            .unwrap_or_default()
            .to_string()
    }

    pub fn set_target_url_parameter(&mut self, target_url_parameter: impl Into<Box<str>>) {
        let target_url_parameter = target_url_parameter.into();
        self.target_url_parameter = if target_url_parameter.is_empty() {
            None
        } else {
            Some(target_url_parameter)
        };
    }

    pub fn set_default_target_url(&mut self, default_target_url: impl Into<Box<str>>) {
        let default_target_url = default_target_url.into();
        assert!(
            !default_target_url.trim().is_empty(),
            "default_target_url cannot be empty"
        );
        self.default_target_url = default_target_url;
    }

    pub fn set_always_use_default_target_url(&mut self, always_use_default_target_url: bool) {
        self.always_use_default_target_url = always_use_default_target_url;
    }

    pub fn is_always_use_default_target_url(&self) -> bool {
        self.always_use_default_target_url
    }

    pub fn default_target_url(&self) -> &str {
        &self.default_target_url
    }

    pub fn determine_target_url(&self, request: &Request) -> String {
        if self.always_use_default_target_url {
            return self.default_target_url.to_string();
        }

        if let Some(parameter) = self.target_url_parameter.as_deref() {
            if let Some(query) = request.uri().query() {
                for pair in query.split('&') {
                    let mut parts = pair.splitn(2, '=');
                    if parts.next() == Some(parameter) {
                        let value = parts.next().unwrap_or_default();
                        if !value.is_empty() {
                            return urlencoding::decode(value)
                                .map(|value| value.into_owned())
                                .unwrap_or_else(|_| value.to_string());
                        }
                    }
                }
            }
        }

        if self.use_referer {
            if let Some(referer) = request.headers().get(axum::http::header::REFERER) {
                if let Ok(referer) = referer.to_str() {
                    if !referer.is_empty() {
                        return referer.to_string();
                    }
                }
            }
        }

        self.default_target_url.to_string()
    }

    pub fn handle(&self, request: &Request, response: &mut axum::response::Response) {
        let target_url = self.determine_target_url(request);
        self.redirect_strategy
            .send_redirect(None, &target_url, response);
    }
}

impl Default for AbstractAuthenticationTargetUrlRequestHandler {
    fn default() -> Self {
        Self {
            default_target_url: "/".into(),
            target_url_parameter: Default::default(),
            always_use_default_target_url: Default::default(),
            use_referer: Default::default(),
            redirect_strategy: Default::default(),
        }
    }
}
