//! The authorities an access-control rule reads are the ones the security
//! context holds.

use std::sync::Arc;

use next_web_core::security::{install_current_authorities, CurrentAuthorities};

use crate::core::{context::SecurityContextHolder, GrantedAuthority};

/// The authorities of the authentication the security context holds.
///
/// The security context is the one the filter chain of the application
/// established for the request, so the rule of a handler reads the authorities
/// the request was authenticated with.
#[derive(Clone, Debug, Default)]
pub struct SecurityContextAuthorities;

impl CurrentAuthorities for SecurityContextAuthorities {
    fn authorities(&self) -> Vec<String> {
        SecurityContextHolder::get_context()
            .get_authentication()
            .map(|authentication| {
                authentication
                    .authorities()
                    .iter()
                    .filter_map(|authority| authority.authority().map(ToString::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Installs the authorities of the security context for the rules to read.
///
/// The security context is read while a request is handled, so installing the
/// source is a one-time step of the startup of an application: it is done when
/// the filter chain of the application is built. Installing it a second time
/// keeps the source that is already installed.
pub fn install_pre_authorize_authorities() {
    if install_current_authorities(Arc::new(SecurityContextAuthorities)).is_err() {
        tracing::debug!(
            "the source of the authorities of the access-control rules is already installed"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use next_web_core::security::CurrentAuthorities;

    use super::SecurityContextAuthorities;
    use crate::{
        authentication::UsernamePasswordAuthenticationToken,
        core::{authority::SimpleGrantedAuthority, context::SecurityContextHolder},
        web::authentication::AuthPrincipal,
    };

    #[test]
    fn the_authorities_are_the_ones_of_the_authentication_of_the_context() {
        let strategy = SecurityContextHolder::get_context_holder_strategy();
        let context = strategy.create_empty_context();
        let authentication = UsernamePasswordAuthenticationToken::authenticated(
            Arc::new("user".to_string()) as AuthPrincipal,
            None,
            vec![
                Arc::new(SimpleGrantedAuthority::new("ROLE_USER")),
                Arc::new(SimpleGrantedAuthority::new("order:read")),
            ],
        );
        context.set_authentication(Some(Arc::new(authentication)));
        strategy.set_context(context);

        let authorities = SecurityContextAuthorities.authorities();

        assert_eq!(
            authorities,
            vec!["ROLE_USER".to_string(), "order:read".to_string()]
        );

        strategy.clear_context();
    }

    #[test]
    fn a_request_without_an_authentication_has_no_authorities() {
        let strategy = SecurityContextHolder::get_context_holder_strategy();
        strategy.clear_context();

        assert!(SecurityContextAuthorities.authorities().is_empty());
    }

    #[test]
    fn two_threads_do_not_share_the_authorities_of_their_request() {
        /// Signs the current thread in with the given authority.
        ///
        /// # Arguments
        ///
        /// * `authority` - The authority of the authentication.
        fn sign_in(authority: &str) {
            let strategy = SecurityContextHolder::get_context_holder_strategy();
            let context = strategy.create_empty_context();
            let authentication = UsernamePasswordAuthenticationToken::authenticated(
                Arc::new("user".to_string()) as AuthPrincipal,
                None,
                vec![Arc::new(SimpleGrantedAuthority::new(authority))],
            );
            context.set_authentication(Some(Arc::new(authentication)));
            strategy.set_context(context);
        }

        // The security context of a request is held per thread, so two requests
        // that are handled at the same time read their own authorities.
        let first = std::thread::spawn(|| {
            sign_in("ROLE_FIRST");
            SecurityContextAuthorities.authorities()
        });
        let second = std::thread::spawn(|| {
            sign_in("ROLE_SECOND");
            SecurityContextAuthorities.authorities()
        });

        assert_eq!(
            first.join().expect("the first thread signed in"),
            vec!["ROLE_FIRST".to_string()]
        );
        assert_eq!(
            second.join().expect("the second thread signed in"),
            vec!["ROLE_SECOND".to_string()]
        );
    }
}
