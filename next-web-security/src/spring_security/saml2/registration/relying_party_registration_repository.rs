use crate::saml2::registration::RelyingPartyRegistration;

/// Reads the relying party registrations of an application.
///
/// The metadata endpoint, the assertion consumer endpoint and the logout
/// endpoint all resolve their registration through this trait, so an
/// application can serve its registrations from wherever it keeps them.
pub trait RelyingPartyRegistrationRepository: Send + Sync {
    /// Returns the registration with the given identifier, if one is registered.
    fn find_by_registration_id(&self, registration_id: &str) -> Option<RelyingPartyRegistration>;

    /// Returns every registered relying party, in registration order.
    fn iter(&self) -> Vec<RelyingPartyRegistration>;
}

/// A [`RelyingPartyRegistrationRepository`] over a list held in memory.
#[derive(Clone, Debug, Default)]
pub struct InMemoryRelyingPartyRegistrationRepository {
    registrations: Vec<RelyingPartyRegistration>,
}

impl InMemoryRelyingPartyRegistrationRepository {
    /// Creates a repository over the given registrations.
    pub fn new(registrations: Vec<RelyingPartyRegistration>) -> Self {
        Self { registrations }
    }
}

impl RelyingPartyRegistrationRepository for InMemoryRelyingPartyRegistrationRepository {
    fn find_by_registration_id(&self, registration_id: &str) -> Option<RelyingPartyRegistration> {
        self.registrations
            .iter()
            .find(|registration| registration.registration_id() == registration_id)
            .cloned()
    }

    fn iter(&self) -> Vec<RelyingPartyRegistration> {
        self.registrations.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::saml2::registration::RelyingPartyRegistration;

    /// Returns a registration with the given identifier.
    fn registration(registration_id: &str) -> RelyingPartyRegistration {
        RelyingPartyRegistration::with_registration_id(registration_id).build()
    }

    #[test]
    fn a_registration_is_found_by_its_identifier() {
        let repository = InMemoryRelyingPartyRegistrationRepository::new(vec![
            registration("okta"),
            registration("azure"),
        ]);

        assert_eq!(
            repository
                .find_by_registration_id("azure")
                .map(|registration| registration.registration_id().to_owned()),
            Some("azure".to_owned())
        );
    }

    #[test]
    fn an_unknown_identifier_is_not_found() {
        let repository =
            InMemoryRelyingPartyRegistrationRepository::new(vec![registration("okta")]);

        assert!(repository.find_by_registration_id("azure").is_none());
    }

    #[test]
    fn the_registrations_are_iterated_in_registration_order() {
        let repository = InMemoryRelyingPartyRegistrationRepository::new(vec![
            registration("okta"),
            registration("azure"),
        ]);

        let identifiers = repository
            .iter()
            .into_iter()
            .map(|registration| registration.registration_id().to_owned())
            .collect::<Vec<_>>();

        assert_eq!(identifiers, vec!["okta".to_owned(), "azure".to_owned()]);
    }
}
