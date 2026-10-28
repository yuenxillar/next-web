use next_web_core::async_trait;
use std::sync::Arc;

use crate::{
    authentication::{
        account_status_user_details_exceptions,
        authentication_event_publisher::{
            AuthenticationEventPublisher, NullAuthenticationEventPublisher,
        },
        authentication_provider::AuthenticationProvider,
    },
    authorization::AuthenticationManager,
    core::{
        Authentication, {AuthenticationError, AuthenticationErrorKind},
    },
};

pub struct ProviderManager {
    providers: Vec<Arc<dyn AuthenticationProvider>>,
    parent: Option<Arc<dyn AuthenticationManager>>,
    event_publisher: Arc<dyn AuthenticationEventPublisher>,
    erase_credentials_after_authentication: bool,
}

impl ProviderManager {
    pub fn new(providers: Vec<Arc<dyn AuthenticationProvider>>) -> Self {
        Self {
            providers,
            parent: None,
            event_publisher: Arc::new(NullAuthenticationEventPublisher),
            erase_credentials_after_authentication: true,
        }
    }

    pub fn with_parent(
        providers: Vec<Arc<dyn AuthenticationProvider>>,
        parent: Arc<dyn AuthenticationManager>,
    ) -> Self {
        Self {
            providers,
            parent: Some(parent),
            event_publisher: Arc::new(NullAuthenticationEventPublisher),
            erase_credentials_after_authentication: true,
        }
    }

    pub fn set_authentication_event_publisher(
        &mut self,
        event_publisher: Arc<dyn AuthenticationEventPublisher>,
    ) {
        self.event_publisher = event_publisher;
    }

    pub fn set_erase_credentials_after_authentication(&mut self, erase: bool) {
        self.erase_credentials_after_authentication = erase;
    }

    pub fn is_erase_credentials_after_authentication(&self) -> bool {
        self.erase_credentials_after_authentication
    }

    pub fn providers(&self) -> &[Arc<dyn AuthenticationProvider>] {
        &self.providers
    }
}

#[async_trait]
impl AuthenticationManager for ProviderManager {
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let auth_type = authentication.of();
        let authentication_arc = authentication.to_builder().build();
        let mut last_exception: Option<AuthenticationError> = None;
        let mut result: Option<Arc<dyn Authentication>> = None;
        let mut parent_result: Option<Arc<dyn Authentication>> = None;

        for provider in &self.providers {
            if !provider.supports(auth_type) {
                continue;
            }

            match provider.authenticate(&authentication_arc).await {
                Ok(auth_result) => {
                    if let Some(auth_result) = auth_result {
                        result = Some(copy_details(authentication, auth_result));
                        break;
                    }
                }
                Err(mut ex) => {
                    ex.set_authentication_request(authentication_arc.clone());
                    match ex.kind() {
                        AuthenticationErrorKind::AccountStatus
                        | AuthenticationErrorKind::InternalService => {
                            prepare_exception(
                                &*self.event_publisher,
                                &ex,
                                authentication_arc.clone(),
                            );
                            return Err(ex);
                        }
                        _ => {
                            last_exception = Some(ex);
                        }
                    }
                }
            }
        }

        // Try parent if no result from providers
        if result.is_none() {
            if let Some(ref parent) = self.parent {
                match parent.authenticate(authentication).await {
                    Ok(auth_result) => {
                        parent_result = Some(auth_result.clone());
                        result = Some(auth_result);
                    }
                    Err(ex) => {
                        if ex.kind() != AuthenticationErrorKind::ProviderNotFound {
                            last_exception = Some(ex);
                        }
                    }
                }
            }
        }

        if let Some(ref auth_result) = result {
            // Credential erasure requires interior mutability (Arc<Mutex<>>) in Rust
            // In a full implementation, tokens needing erasure would use Arc<Mutex<dyn Authentication>>
            let returned = if self.erase_credentials_after_authentication {
                let mut builder = auth_result.to_builder();
                builder.credentials(None);
                builder.build()
            } else {
                auth_result.clone()
            };

            // Publish success event (only if parent didn't already do it)
            if parent_result.is_none() {
                self.event_publisher
                    .publish_authentication_success(returned.clone());
            }

            return Ok(returned);
        }

        // No result - prepare and throw the last exception
        let final_exception = last_exception.unwrap_or_else(|| {
            account_status_user_details_exceptions::provider_not_found(format!(
                "No AuthenticationProvider found for {:?}",
                auth_type
            ))
        });

        prepare_exception(&*self.event_publisher, &final_exception, authentication_arc);

        Err(final_exception)
    }
}

fn copy_details(
    source: &dyn Authentication,
    dest: Arc<dyn Authentication>,
) -> Arc<dyn Authentication> {
    if source.details().is_none() || dest.details().is_some() {
        return dest;
    }
    let mut builder = dest.to_builder();
    builder.details(source.details().cloned());
    builder.build()
}

fn prepare_exception(
    event_publisher: &dyn AuthenticationEventPublisher,
    ex: &AuthenticationError,
    authentication: Arc<dyn Authentication>,
) {
    event_publisher.publish_authentication_failure(ex.clone(), authentication);
}
