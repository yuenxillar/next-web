//! The credentials a relying party holds for the users it registered.
//!
//! A credential outlives the ceremony that created it: the public key is what
//! every later authentication is verified against, and the sign count is what
//! makes a cloned authenticator visible. Both live here, keyed by the credential
//! id the authenticator chose.

use std::collections::HashMap;
use std::sync::RwLock;

/// A credential the relying party holds for a user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    credential_id: Vec<u8>,
    user_name: String,
    public_key: Vec<u8>,
    algorithm: i64,
    sign_count: u32,
}

impl Credential {
    /// Creates a credential with the sign count an authenticator starts at.
    pub fn new(
        credential_id: impl Into<Vec<u8>>,
        user_name: impl Into<String>,
        public_key: impl Into<Vec<u8>>,
        algorithm: i64,
    ) -> Self {
        Self {
            credential_id: credential_id.into(),
            user_name: user_name.into(),
            public_key: public_key.into(),
            algorithm,
            sign_count: 0,
        }
    }

    /// Returns the identifier the authenticator chose for the credential.
    pub fn credential_id(&self) -> &[u8] {
        &self.credential_id
    }

    /// Returns the name of the user the credential belongs to.
    pub fn user_name(&self) -> &str {
        &self.user_name
    }

    /// Returns the public key of the credential, as the authenticator submitted
    /// it.
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    /// Returns the identifier of the signature algorithm of the credential.
    pub fn algorithm(&self) -> i64 {
        self.algorithm
    }

    /// Returns how many times the credential was used.
    pub fn sign_count(&self) -> u32 {
        self.sign_count
    }

    /// Records a use of the credential.
    pub fn set_sign_count(&mut self, sign_count: u32) {
        self.sign_count = sign_count;
    }
}

/// The credentials of the users of a relying party.
pub trait UserCredentialRepository: Send + Sync {
    /// Stores a credential, or replaces the one with the same id.
    fn save(&self, credential: Credential);

    /// Returns the credential with the given id.
    fn find_by_credential_id(&self, credential_id: &[u8]) -> Option<Credential>;

    /// Returns every credential the given user registered.
    ///
    /// A ceremony is restricted to the credentials of the user who starts it, so
    /// the list of them is what the browser is offered to choose from.
    fn find_by_user_name(&self, user_name: &str) -> Vec<Credential>;

    /// Records the sign count of a use, and returns whether the credential is
    /// known.
    fn update_sign_count(&self, credential_id: &[u8], sign_count: u32) -> bool;
}

/// A [`UserCredentialRepository`] that holds the credentials in memory.
#[derive(Debug, Default)]
pub struct InMemoryUserCredentialRepository {
    credentials: RwLock<HashMap<Vec<u8>, Credential>>,
}

impl UserCredentialRepository for InMemoryUserCredentialRepository {
    fn save(&self, credential: Credential) {
        let mut credentials = self
            .credentials
            .write()
            .expect("the credential store lock is poisoned");
        credentials.insert(credential.credential_id.clone(), credential);
    }

    fn find_by_credential_id(&self, credential_id: &[u8]) -> Option<Credential> {
        let credentials = self
            .credentials
            .read()
            .expect("the credential store lock is poisoned");

        credentials.get(credential_id).cloned()
    }

    fn find_by_user_name(&self, user_name: &str) -> Vec<Credential> {
        let credentials = self
            .credentials
            .read()
            .expect("the credential store lock is poisoned");

        credentials
            .values()
            .filter(|credential| credential.user_name == user_name)
            .cloned()
            .collect()
    }

    fn update_sign_count(&self, credential_id: &[u8], sign_count: u32) -> bool {
        let mut credentials = self
            .credentials
            .write()
            .expect("the credential store lock is poisoned");

        match credentials.get_mut(credential_id) {
            Some(credential) => {
                credential.set_sign_count(sign_count);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a credential of the user `Max`.
    fn credential() -> Credential {
        Credential::new(b"the-credential".to_vec(), "Max", b"the-public-key".to_vec(), -7)
    }

    #[test]
    fn a_stored_credential_is_found_by_its_id() {
        let repository = InMemoryUserCredentialRepository::default();
        repository.save(credential());

        let found = repository
            .find_by_credential_id(b"the-credential")
            .expect("the credential is stored");

        assert_eq!(found.user_name(), "Max");
        assert_eq!(found.algorithm(), -7);
        assert_eq!(found.sign_count(), 0);
    }

    #[test]
    fn an_unknown_credential_is_not_found() {
        let repository = InMemoryUserCredentialRepository::default();

        assert_eq!(repository.find_by_credential_id(b"another-credential"), None);
    }

    #[test]
    fn the_credentials_of_a_user_are_found_by_the_name_of_the_user() {
        let repository = InMemoryUserCredentialRepository::default();
        repository.save(credential());
        repository.save(Credential::new(
            b"another-credential".to_vec(),
            "Bob",
            b"another-public-key".to_vec(),
            -257,
        ));

        let credentials = repository.find_by_user_name("Max");

        assert_eq!(credentials.len(), 1, "only the credentials of Max");
        assert_eq!(credentials[0].credential_id(), b"the-credential");
    }

    #[test]
    fn a_user_without_a_credential_has_none() {
        let repository = InMemoryUserCredentialRepository::default();
        repository.save(credential());

        assert!(repository.find_by_user_name("Bob").is_empty());
    }

    #[test]
    fn the_sign_count_of_a_used_credential_is_recorded() {
        let repository = InMemoryUserCredentialRepository::default();
        repository.save(credential());

        assert!(repository.update_sign_count(b"the-credential", 7));

        let found = repository
            .find_by_credential_id(b"the-credential")
            .expect("the credential is stored");
        assert_eq!(found.sign_count(), 7, "the counter of the authenticator is kept");
    }

    #[test]
    fn the_sign_count_of_an_unknown_credential_is_not_recorded() {
        let repository = InMemoryUserCredentialRepository::default();

        assert!(!repository.update_sign_count(b"another-credential", 7));
    }
}
