use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use next_web_core::traits::http::{http_request::HttpRequest, HttpSession};

use crate::web::authentication::session::{
    BaseSessionFixationProtectionStrategy, BaseSessionFixationProtectionStrategyExt,
};

/// Uses HttpServletRequest.changeSessionId() to protect against session fixation attacks.
/// This is the default implementation.
#[derive(Clone)]
pub struct ChangeSessionIdAuthenticationStrategy(BaseSessionFixationProtectionStrategy);

impl BaseSessionFixationProtectionStrategyExt for ChangeSessionIdAuthenticationStrategy {
    fn apply_session_fixation<'a>(
        &'a self,
        request: &'a mut dyn HttpRequest,
    ) -> Option<&'a mut dyn HttpSession> {
        request.change_session_id();
        request.session_mut(true)
    }
}

impl Deref for ChangeSessionIdAuthenticationStrategy {
    type Target = BaseSessionFixationProtectionStrategy;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChangeSessionIdAuthenticationStrategy {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Debug for ChangeSessionIdAuthenticationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeSessionIdAuthenticationStrategy")
            .finish()
    }
}

impl Default for ChangeSessionIdAuthenticationStrategy {
    fn default() -> Self {
        Self(Default::default())
    }
}
