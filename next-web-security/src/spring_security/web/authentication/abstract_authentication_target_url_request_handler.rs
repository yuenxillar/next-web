use std::sync::Arc;

use crate::web::redirect_strategy::DefaultRedirectStrategy;

pub struct AbstractAuthenticationTargetUrlRequestHandler {
    target_url_parameter: Option<Box<str>>,
    default_target_url: Box<str>,

    always_use_default_target_url: bool,
    use_referer: bool,

    redirect_strategy: DefaultRedirectStrategy,
}

impl AbstractAuthenticationTargetUrlRequestHandler {
    pub fn get_target_url_parameter(&self) -> String {
        todo!()
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
