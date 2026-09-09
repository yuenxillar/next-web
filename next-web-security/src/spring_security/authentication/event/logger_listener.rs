use next_web_context::ApplicationEvent;
use tracing::{info, warn};

use crate::{
    authentication::event::{
        AuthenticationFailureBadCredentialsEvent, AuthenticationFailureCredentialsExpiredEvent,
        AuthenticationFailureDisabledEvent, AuthenticationFailureExpiredEvent,
        AuthenticationFailureLockedEvent, AuthenticationFailureProviderNotFoundEvent,
        AuthenticationFailureProxyUntrustedEvent, AuthenticationFailureServiceErrorEvent,
    },
    core::Authentication,
};

/// Logs authentication events using the event's concrete type and principal name.
#[derive(Clone, Default)]
pub struct LoggerListener;

impl LoggerListener {
    pub fn on_application_event(&self, event: &dyn ApplicationEvent) {
        let authentication = event
            .source()
            .downcast_ref::<std::sync::Arc<dyn Authentication>>();
        let principal = authentication
            .map(|authentication| authentication.name().into_owned())
            .unwrap_or_default();

        let event_type = event.event_type();
        let is_failure = event_type
            == std::any::TypeId::of::<AuthenticationFailureBadCredentialsEvent>()
            || event_type == std::any::TypeId::of::<AuthenticationFailureCredentialsExpiredEvent>()
            || event_type == std::any::TypeId::of::<AuthenticationFailureDisabledEvent>()
            || event_type == std::any::TypeId::of::<AuthenticationFailureExpiredEvent>()
            || event_type == std::any::TypeId::of::<AuthenticationFailureLockedEvent>()
            || event_type == std::any::TypeId::of::<AuthenticationFailureProviderNotFoundEvent>()
            || event_type == std::any::TypeId::of::<AuthenticationFailureProxyUntrustedEvent>()
            || event_type == std::any::TypeId::of::<AuthenticationFailureServiceErrorEvent>();

        if is_failure {
            warn!(
                event = ?event_type,
                principal,
                "Authentication failure"
            );
        } else {
            info!(event = ?event_type, principal, "Authentication event");
        }
    }
}
