use std::{
    fmt,
    sync::{Arc, RwLock},
};

use crate::core::{context::SecurityContext, Authentication};

#[derive(Default, Clone)]
pub struct SecurityContextImpl {
    authentication: Arc<RwLock<Option<Arc<dyn Authentication>>>>,
}

impl SecurityContextImpl {
    pub fn new(authentication: Arc<dyn Authentication>) -> Self {
        Self {
            authentication: Arc::new(RwLock::new(Some(authentication))),
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    fn current_authentication(&self) -> Option<Arc<dyn Authentication>> {
        self.authentication
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }
}

impl SecurityContext for SecurityContextImpl {
    fn get_authentication(&self) -> Option<Arc<dyn Authentication>> {
        self.current_authentication()
    }

    fn set_authentication(&self, authentication: Option<Arc<dyn Authentication>>) {
        if let Ok(mut guard) = self.authentication.write() {
            *guard = authentication;
        }
    }
}

impl PartialEq for SecurityContextImpl {
    fn eq(&self, other: &Self) -> bool {
        self.get_authentication().as_ref().map(|auth| auth.name())
            == other.get_authentication().as_ref().map(|auth| auth.name())
            && self
                .get_authentication()
                .as_ref()
                .map(|auth| auth.is_authenticated())
                == other
                    .get_authentication()
                    .as_ref()
                    .map(|auth| auth.is_authenticated())
    }
}

impl Eq for SecurityContextImpl {}

impl fmt::Debug for SecurityContextImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get_authentication() {
            Some(authentication) => write!(
                f,
                "SecurityContextImpl [Authentication={}, Authenticated={}]",
                authentication.name(),
                authentication.is_authenticated()
            ),
            None => write!(f, "SecurityContextImpl [Null authentication]"),
        }
    }
}

impl fmt::Display for SecurityContextImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
