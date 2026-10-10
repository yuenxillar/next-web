//! The seam between an access-control rule and the authentication of the
//! current request.
//!
//! A rule such as `has_role('ADMIN')` is evaluated while a request is handled,
//! and it has to know the authorities the request was authenticated with. The
//! framework cannot read them itself: the authentication lives in the crate
//! that implements it. A crate that authenticates requests therefore installs a
//! [`CurrentAuthorities`] once, and the rules of the framework read it.

use std::sync::{Arc, OnceLock};

/// Returns the authorities of the request that is being handled.
///
/// The implementation reads the authentication of the current request, so it is
/// called while that request is handled.
pub trait CurrentAuthorities: Send + Sync {
    /// Returns the authority strings the current request was authenticated
    /// with.
    ///
    /// A request without an authentication answers with no authority, so a rule
    /// that asks for a role or an authority of it does not hold.
    fn authorities(&self) -> Vec<String>;
}

/// The authorities source the rules read.
static AUTHORITIES: OnceLock<Arc<dyn CurrentAuthorities>> = OnceLock::new();

/// Installs the source the access-control rules read.
///
/// The source is installed once per process, so a later call reports the source
/// that is already installed. A rule that is evaluated before a source was
/// installed reads no authority, which is what keeps an application without
/// authentication from allowing a rule by accident.
///
/// # Arguments
///
/// * `authorities` - The source of the authorities of a request.
pub fn install_current_authorities(
    authorities: Arc<dyn CurrentAuthorities>,
) -> Result<(), Arc<dyn CurrentAuthorities>> {
    AUTHORITIES.set(authorities)
}

/// Returns the authorities of the request that is being handled.
///
/// The authorities are the ones the installed [`CurrentAuthorities`] reports,
/// and they are empty when no source was installed.
pub fn current_authorities() -> Vec<String> {
    AUTHORITIES
        .get()
        .map(|authorities| authorities.authorities())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{current_authorities, install_current_authorities, CurrentAuthorities};

    /// A source that answers a fixed authority.
    struct FixedAuthorities;

    impl CurrentAuthorities for FixedAuthorities {
        fn authorities(&self) -> Vec<String> {
            vec!["ROLE_ADMIN".to_string()]
        }
    }

    #[test]
    fn a_rule_reads_the_authorities_of_the_installed_source() {
        // The source is installed once for the whole test binary, and no other
        // test of the crate installs one, so the authorities of this source are
        // the ones a rule reads afterwards.
        let installed = install_current_authorities(Arc::new(FixedAuthorities));
        assert!(
            installed.is_ok(),
            "no other source of the tests was installed"
        );

        assert_eq!(
            current_authorities(),
            vec!["ROLE_ADMIN".to_string()],
            "the source reports the authorities of the request"
        );
    }
}
