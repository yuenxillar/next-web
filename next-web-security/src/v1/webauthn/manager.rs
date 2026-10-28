//! The two ceremonies of a relying party.
//!
//! A registration turns the attestation a browser submits into a credential the
//! relying party keeps. An authentication turns the assertion it submits into
//! the credential that made it. Both walk the same chain of checks in the same
//! order — the ceremony the client data names, the challenge this relying party
//! issued, the origin it allows, the relying party the authenticator data binds
//! to, and the user who took part — and only then does the signature or the
//! public key matter.

use std::sync::Arc;

use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};

use super::{
    attestation::AttestationObject,
    authenticator_data::AuthenticatorData,
    challenge::{ChallengeRepository, InMemoryChallengeRepository},
    client_data::{ClientData, CREATE, GET},
    cose::CosePublicKey,
    credential::{Credential, InMemoryUserCredentialRepository, UserCredentialRepository},
    error::{WebAuthnError, WebAuthnErrorKind},
    options::{
        PublicKeyCredentialCreationOptions, PublicKeyCredentialDescriptor,
        PublicKeyCredentialRequestOptions, PublicKeyCredentialUserEntity, RelyingParty,
    },
    request::{AuthenticationRequest, RegistrationRequest},
};

/// The number of random bytes of a challenge.
///
/// The specification asks for at least sixteen; thirty-two is what a SHA-256
/// hash is wide and what the rest of this port already uses for a nonce.
const CHALLENGE_LENGTH: usize = 32;

/// The tag the challenge of a registration is stored under.
const CREATE_CHALLENGE: u8 = 0x01;

/// The tag the challenge of an authentication is stored under.
const GET_CHALLENGE: u8 = 0x02;

/// The ceremonies of one relying party.
///
/// A manager holds what a ceremony needs between its two halves — the challenge
/// it issued and the credentials it registered — and the settings that decide
/// what it accepts: the relying party the credentials are bound to, the origins
/// a ceremony may run on, and whether a verified user is required.
#[derive(Clone)]
pub struct WebAuthnManager {
    relying_party: RelyingParty,
    allowed_origins: Vec<String>,
    require_user_verification: bool,
    challenge_ttl: Duration,
    challenges: Arc<dyn ChallengeRepository>,
    credentials: Arc<dyn UserCredentialRepository>,
}

impl WebAuthnManager {
    /// Creates a manager for a relying party, over in-memory stores.
    pub fn new(relying_party: RelyingParty) -> Self {
        Self {
            relying_party,
            allowed_origins: Vec::new(),
            require_user_verification: false,
            challenge_ttl: Duration::minutes(5),
            challenges: Arc::new(InMemoryChallengeRepository::default()),
            credentials: Arc::new(InMemoryUserCredentialRepository::default()),
        }
    }

    /// Sets the origins a ceremony may run on.
    ///
    /// A relying party that allows no origin refuses every ceremony: a browser
    /// reports the origin it ran on, and a ceremony whose origin is not one of
    /// these is a ceremony of another site.
    pub fn allowed_origins<I, S>(&mut self, origins: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_origins = origins.into_iter().map(Into::into).collect();
        self
    }

    /// Sets whether the ceremony requires the authenticator to verify the user.
    pub fn require_user_verification(&mut self, require_user_verification: bool) -> &mut Self {
        self.require_user_verification = require_user_verification;
        self
    }

    /// Sets how long a challenge may be answered.
    pub fn challenge_ttl(&mut self, challenge_ttl: Duration) -> &mut Self {
        self.challenge_ttl = challenge_ttl;
        self
    }

    /// Sets where the challenges a ceremony answers are remembered.
    pub fn challenge_repository(&mut self, challenges: Arc<dyn ChallengeRepository>) -> &mut Self {
        self.challenges = challenges;
        self
    }

    /// Sets where the credentials are kept.
    pub fn credential_repository(
        &mut self,
        credentials: Arc<dyn UserCredentialRepository>,
    ) -> &mut Self {
        self.credentials = credentials;
        self
    }

    /// Returns the relying party a ceremony runs for.
    pub fn relying_party(&self) -> &RelyingParty {
        &self.relying_party
    }

    /// Returns the origins a ceremony may run on.
    pub fn allowed_origin_list(&self) -> &[String] {
        &self.allowed_origins
    }

    /// Starts a registration ceremony for a user.
    ///
    /// The user is identified by name: the identifier the authenticator stores
    /// as the user handle is the UTF-8 of that name, so that the handle a later
    /// authentication reports can be checked against the credential.
    pub fn create_registration_options(
        &self,
        user_name: &str,
        display_name: &str,
    ) -> PublicKeyCredentialCreationOptions {
        let options = PublicKeyCredentialCreationOptions::new(
            self.new_challenge(CREATE_CHALLENGE),
            self.relying_party.clone(),
            PublicKeyCredentialUserEntity::new(
                user_name.as_bytes().to_vec(),
                user_name,
                display_name,
            ),
        );
        // A ceremony that enforces a verified user has to ask the browser for
        // one; without the request, an authenticator is free to skip it.
        let options = match self.user_verification() {
            Some(user_verification) => options.user_verification(user_verification),
            None => options,
        };

        let registered = self.credentials.find_by_user_name(user_name);
        if registered.is_empty() {
            return options;
        }

        // An authenticator that already has a credential of this user is told
        // not to make a second one, which is what keeps one account from piling
        // up credentials on the same device.
        options.exclude_credentials(
            registered
                .iter()
                .map(|credential| {
                    PublicKeyCredentialDescriptor::new(credential.credential_id().to_vec())
                })
                .collect(),
        )
    }

    /// Finishes a registration ceremony, and stores the credential.
    pub fn register(
        &self,
        user_name: &str,
        request: &RegistrationRequest,
    ) -> Result<Credential, WebAuthnError> {
        let attestation = AttestationObject::parse(request.attestation_object())?;
        attestation.validate()?;

        let client_data = ClientData::parse(request.client_data_json())?;
        self.check_client_data(&client_data, CREATE)?;
        self.consume_challenge(CREATE_CHALLENGE, &client_data)?;

        let authenticator_data = attestation.authenticator_data();
        authenticator_data.validate(&self.rp_id_hash(), self.require_user_verification)?;

        let attested = attestation.attested_credential()?;
        if attested.credential_id() != request.credential_id() {
            return Err(WebAuthnError::new(
                WebAuthnErrorKind::Credential,
                "the registration request names a credential that is not the one the attestation \
                 carries",
            ));
        }

        // A credential id is stored under one user. A registration that names
        // the credential of another user would replace the key that user signs
        // with, which is a lockout of that user and not a registration of this
        // one.
        if let Some(existing) = self
            .credentials
            .find_by_credential_id(attested.credential_id())
        {
            if existing.user_name() != user_name {
                return Err(WebAuthnError::new(
                    WebAuthnErrorKind::Credential,
                    "the credential id is already registered for another user, and a registration \
                     cannot take it over",
                ));
            }
        }

        let mut credential = Credential::new(
            attested.credential_id().to_vec(),
            user_name,
            attested.cose_public_key().to_vec(),
            attested.public_key().algorithm(),
        );
        credential.set_sign_count(authenticator_data.sign_count());
        self.credentials.save(credential.clone());

        Ok(credential)
    }

    /// Starts an authentication ceremony for a user.
    ///
    /// The browser is offered the credentials the user registered, so that an
    /// authenticator which holds none of them cannot answer.
    pub fn create_authentication_options(
        &self,
        user_name: &str,
    ) -> PublicKeyCredentialRequestOptions {
        let options = PublicKeyCredentialRequestOptions::new(self.new_challenge(GET_CHALLENGE))
            .rp_id(self.relying_party.id());
        let options = match self.user_verification() {
            Some(user_verification) => options.user_verification(user_verification),
            None => options,
        };
        let registered = self.credentials.find_by_user_name(user_name);
        if registered.is_empty() {
            return options;
        }

        options.allow_credentials(
            registered
                .iter()
                .map(|credential| {
                    PublicKeyCredentialDescriptor::new(credential.credential_id().to_vec())
                })
                .collect(),
        )
    }

    /// Finishes an authentication ceremony, and returns the credential that
    /// made the assertion.
    ///
    /// The credential carries the user it was registered for, and that is the
    /// user this reports: an assertion says which credential was used, and the
    /// credential says who owns it. A caller that started a ceremony for one
    /// named user has to compare [`Credential::user_name`] with that name —
    /// [`PublicKeyCredentialRequestOptions::allow_credentials`] is a hint to the
    /// browser, not a check the relying party can rely on.
    pub fn authenticate(
        &self,
        request: &AuthenticationRequest,
    ) -> Result<Credential, WebAuthnError> {
        let client_data = ClientData::parse(request.client_data_json())?;
        self.check_client_data(&client_data, GET)?;
        self.consume_challenge(GET_CHALLENGE, &client_data)?;

        let authenticator_data = AuthenticatorData::parse(request.authenticator_data())?;
        authenticator_data.validate(&self.rp_id_hash(), self.require_user_verification)?;

        let Some(mut credential) = self
            .credentials
            .find_by_credential_id(request.credential_id())
        else {
            return Err(WebAuthnError::new(
                WebAuthnErrorKind::Credential,
                "no credential of this relying party has the id the ceremony names",
            ));
        };

        if let Some(user_handle) = request.user_handle() {
            if user_handle != credential.user_name().as_bytes() {
                return Err(WebAuthnError::new(
                    WebAuthnErrorKind::Credential,
                    "the credential belongs to another user than the user handle of the ceremony \
                     names",
                ));
            }
        }

        // What the signature covers is the authenticator data followed by the
        // hash of the client data: the two halves of the ceremony, tied together
        // by the key, and by nothing else.
        let mut message = request.authenticator_data().to_vec();
        message.extend_from_slice(&Sha256::digest(request.client_data_json()));
        CosePublicKey::parse(credential.public_key())?.verify(&message, request.signature())?;

        let sign_count = authenticator_data.sign_count();
        let previous = credential.sign_count();
        if (previous != 0 || sign_count != 0) && sign_count <= previous {
            return Err(WebAuthnError::new(
                WebAuthnErrorKind::Credential,
                format!(
                    "the credential reported {previous} uses and this ceremony reports {sign_count}: \
                     a counter that does not grow is the signal of a cloned credential"
                ),
            ));
        }

        credential.set_sign_count(sign_count);
        self.credentials
            .update_sign_count(credential.credential_id(), sign_count);

        Ok(credential)
    }

    /// Returns the user verification the options of a ceremony ask for.
    fn user_verification(&self) -> Option<&'static str> {
        self.require_user_verification.then_some("required")
    }

    /// Checks the ceremony and the origin of the client data.
    fn check_client_data(
        &self,
        client_data: &ClientData,
        kind: &str,
    ) -> Result<(), WebAuthnError> {
        if self.allowed_origins.is_empty() {
            return Err(WebAuthnError::new(
                WebAuthnErrorKind::Configuration,
                "no origin is allowed to run a ceremony: configure the origins of the relying \
                 party before a ceremony can be verified",
            ));
        }

        client_data.validate(kind, &self.allowed_origins)?;

        Ok(())
    }

    /// Consumes the challenge the client data answers.
    ///
    /// The store is what decides, not a string the caller already holds: a
    /// challenge is accepted once, in the window it was issued for, and only by
    /// the ceremony that issued it.
    fn consume_challenge(&self, tag: u8, client_data: &ClientData) -> Result<(), WebAuthnError> {
        let challenge = client_data.challenge_bytes()?;
        if !self.challenges.consume(&Self::challenge_key(tag, &challenge)) {
            return Err(WebAuthnError::new(
                WebAuthnErrorKind::Challenge,
                "the challenge of the ceremony was not issued by this relying party to this \
                 ceremony, or it was answered already",
            ));
        }

        Ok(())
    }

    /// Issues a fresh challenge for one ceremony, remembers it, and returns it.
    fn new_challenge(&self, tag: u8) -> Vec<u8> {
        let mut challenge = [0u8; CHALLENGE_LENGTH];
        for byte in &mut challenge {
            *byte = rand::random();
        }
        self.challenges
            .save(&Self::challenge_key(tag, &challenge), Utc::now() + self.challenge_ttl);

        challenge.to_vec()
    }

    /// Returns the key the challenge store holds a challenge under.
    ///
    /// The store is a map of bytes to an expiry, so the ceremony the challenge
    /// was issued for is part of the key: a challenge handed out to start an
    /// authentication cannot answer a registration.
    fn challenge_key(tag: u8, challenge: &[u8]) -> Vec<u8> {
        let mut key = Vec::with_capacity(1 + challenge.len());
        key.push(tag);
        key.extend_from_slice(challenge);

        key
    }

    /// Returns the hash the authenticator data binds the credential to.
    fn rp_id_hash(&self) -> Vec<u8> {
        Sha256::digest(self.relying_party.id().as_bytes()).to_vec()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Duration;

    use super::*;
    use crate::webauthn::test_support::{
        attestation_object, authenticator_data, authentication_submission, client_data_json,
        cose_key_of, registration_submission, sign, signing_key,
        ATTESTED_CREDENTIAL_DATA, USER_PRESENT,
    };

    /// The origin every ceremony of the tests runs on.
    const ORIGIN: &str = "https://sp.example.org";

    /// The identifier of the relying party of the tests.
    const RP_ID: &str = "sp.example.org";

    /// The relying party of the tests.
    fn relying_party() -> RelyingParty {
        RelyingParty::new(RP_ID, "The application")
    }

    /// Returns a manager over in-memory stores the test can look into.
    fn fixture() -> (WebAuthnManager, Arc<InMemoryUserCredentialRepository>) {
        let credentials = Arc::new(InMemoryUserCredentialRepository::default());
        let mut manager = WebAuthnManager::new(relying_party());
        manager.allowed_origins([ORIGIN]);
        manager.credential_repository(credentials.clone());

        (manager, credentials)
    }

    /// Registers `the-credential` for the given user, and returns the credential.
    fn register(
        manager: &WebAuthnManager,
        credential_id: &[u8],
        user_name: &str,
        sign_count: u32,
    ) -> crate::webauthn::Credential {
        let key = signing_key();
        let options = manager.create_registration_options(user_name, "Max");
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            sign_count,
            Some((credential_id, &cose_key)),
        );
        let submission = registration_submission(
            credential_id,
            &client_data_json(CREATE, options.challenge(), ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        manager
            .register(user_name, &request)
            .expect("the registration holds")
    }

    /// Returns the submission of an authentication of `the-credential`.
    fn authentication(
        manager: &WebAuthnManager,
        credential_id: &[u8],
        sign_count: u32,
        user_handle: Option<&[u8]>,
    ) -> Vec<u8> {
        let key = signing_key();
        let options = manager.create_authentication_options("Max");
        let client_data = client_data_json(GET, options.challenge(), ORIGIN, false);
        let authenticator_data =
            authenticator_data(RP_ID, USER_PRESENT, sign_count, None);
        let mut message = authenticator_data.clone();
        message.extend_from_slice(&sha256(&client_data));
        let signature = sign(&key, &message);

        authentication_submission(
            credential_id,
            &client_data,
            &authenticator_data,
            &signature,
            user_handle,
        )
    }

    /// Returns the SHA-256 of a message, which a ceremony signs over.
    fn sha256(message: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};

        Sha256::digest(message).to_vec()
    }

    #[test]
    fn the_options_of_a_registration_carry_the_challenge_the_manager_remembers() {
        let (manager, _) = fixture();

        let options = manager.create_registration_options("Max", "Max Mustermann");
        let json = serde_json::to_value(&options).expect("the options serialize");

        assert!(!options.challenge().is_empty(), "a challenge was issued");
        assert_eq!(json["rp"]["id"], RP_ID);
        assert_eq!(json["user"]["name"], "Max");
        assert_eq!(
            json["user"]["id"],
            crate::webauthn::test_support::to_base64_url(b"Max"),
            "the user handle is the name the relying party identifies the user by"
        );
    }

    #[test]
    fn a_registration_that_holds_every_condition_stores_the_credential() {
        let (manager, credentials) = fixture();

        let credential = register(&manager, b"the-credential", "Max", 0);

        assert_eq!(credential.user_name(), "Max");
        assert_eq!(credential.algorithm(), -7, "ES256");
        assert_eq!(
            credential.public_key(),
            cose_key_of(&signing_key()).as_slice()
        );

        let stored = credentials
            .find_by_credential_id(b"the-credential")
            .expect("the credential is stored");
        assert_eq!(stored.user_name(), "Max");
    }

    #[test]
    fn a_registration_of_another_origin_is_refused() {
        let (manager, credentials) = fixture();
        let key = signing_key();
        let options = manager.create_registration_options("Max", "Max");
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, options.challenge(), "https://evil.example.org", false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Max", &request)
            .expect_err("the ceremony did not run on the origin of the relying party");

        assert_eq!(error.kind(), WebAuthnErrorKind::ClientData);
        assert!(error.message().contains("evil.example.org"), "was: {error}");
        assert_eq!(
            credentials.find_by_credential_id(b"the-credential"),
            None,
            "nothing is stored when the ceremony is refused"
        );
    }

    #[test]
    fn a_cross_origin_registration_is_refused() {
        let (manager, _) = fixture();
        let key = signing_key();
        let options = manager.create_registration_options("Max", "Max");
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, options.challenge(), ORIGIN, true),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Max", &request)
            .expect_err("a cross-origin frame may not run the ceremony");

        assert_eq!(error.kind(), WebAuthnErrorKind::ClientData);
    }

    #[test]
    fn a_registration_of_another_relying_party_is_refused() {
        let (manager, _) = fixture();
        let key = signing_key();
        let options = manager.create_registration_options("Max", "Max");
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            "evil.example.org",
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, options.challenge(), ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Max", &request)
            .expect_err("the credential belongs to another relying party");

        assert_eq!(error.kind(), WebAuthnErrorKind::AuthenticatorData);
        assert!(
            error.message().contains("another relying party"),
            "was: {error}"
        );
    }

    #[test]
    fn a_registration_that_answers_a_challenge_that_was_never_issued_is_refused() {
        let (manager, _) = fixture();
        let key = signing_key();
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, b"a-challenge-nobody-issued", ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Max", &request)
            .expect_err("the challenge was never issued");

        assert_eq!(error.kind(), WebAuthnErrorKind::Challenge);
    }

    #[test]
    fn a_registration_that_answers_the_same_challenge_twice_is_refused() {
        let (manager, _) = fixture();
        let key = signing_key();
        let options = manager.create_registration_options("Max", "Max");
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, options.challenge(), ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        manager
            .register("Max", &request)
            .expect("the first answer of the challenge holds");
        let error = manager
            .register("Max", &request)
            .expect_err("the challenge was used up by the first answer");

        assert_eq!(error.kind(), WebAuthnErrorKind::Challenge);
    }

    #[test]
    fn a_registration_of_an_attestation_that_is_not_none_is_refused() {
        let (manager, _) = fixture();
        let key = signing_key();
        let options = manager.create_registration_options("Max", "Max");
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, options.challenge(), ORIGIN, false),
            &attestation_object("packed", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Max", &request)
            .expect_err("this port cannot verify a packed attestation");

        assert_eq!(error.kind(), WebAuthnErrorKind::Attestation);
        assert!(error.message().contains("packed"), "was: {error}");
    }

    #[test]
    fn a_registration_without_the_user_present_flag_is_refused() {
        let (manager, _) = fixture();
        let key = signing_key();
        let options = manager.create_registration_options("Max", "Max");
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, options.challenge(), ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Max", &request)
            .expect_err("the user did not take part");

        assert_eq!(error.kind(), WebAuthnErrorKind::AuthenticatorData);
        assert!(error.message().contains("did not take part"), "was: {error}");
    }

    #[test]
    fn a_ceremony_that_requires_a_verified_user_asks_the_browser_for_one() {
        let (mut manager, _) = fixture();
        manager.require_user_verification(true);

        let registration = manager.create_registration_options("Max", "Max");
        let json = serde_json::to_value(&registration).expect("the options serialize");
        assert_eq!(
            json["userVerification"], "required",
            "a ceremony that enforces a verified user has to ask for one"
        );

        let authentication = manager.create_authentication_options("Max");
        let json = serde_json::to_value(&authentication).expect("the options serialize");
        assert_eq!(json["userVerification"], "required");
    }

    #[test]
    fn an_authentication_of_an_unverified_user_is_refused_when_verification_is_required() {
        let (mut manager, _) = fixture();
        register(&manager, b"the-credential", "Max", 0);
        manager.require_user_verification(true);
        let submission = authentication(&manager, b"the-credential", 1, None);
        let request = AuthenticationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .authenticate(&request)
            .expect_err("the ceremony requires a verified user");

        assert_eq!(error.kind(), WebAuthnErrorKind::AuthenticatorData);
        assert!(error.message().contains("verified user"), "was: {error}");
    }

    #[test]
    fn an_authentication_of_a_verified_user_holds_when_verification_is_required() {
        use crate::webauthn::test_support::USER_VERIFIED;

        let (mut manager, _) = fixture();
        register(&manager, b"the-credential", "Max", 0);
        manager.require_user_verification(true);
        let key = signing_key();
        let options = manager.create_authentication_options("Max");
        let client_data = client_data_json(GET, options.challenge(), ORIGIN, false);
        let authenticator_data =
            authenticator_data(RP_ID, USER_PRESENT | USER_VERIFIED, 1, None);
        let mut message = authenticator_data.clone();
        message.extend_from_slice(&sha256(&client_data));
        let submission = authentication_submission(
            b"the-credential",
            &client_data,
            &authenticator_data,
            &sign(&key, &message),
            None,
        );
        let request = AuthenticationRequest::parse(&submission).expect("the request parses");

        let credential = manager
            .authenticate(&request)
            .expect("the authenticator verified the user");

        assert_eq!(credential.user_name(), "Max");
    }

    #[test]
    fn a_challenge_issued_to_an_authentication_does_not_answer_a_registration() {
        let (manager, _) = fixture();
        // A ceremony the relying party started for an authentication, whose
        // challenge is then answered by a registration.
        let options = manager.create_authentication_options("Max");
        let key = signing_key();
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, options.challenge(), ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Max", &request)
            .expect_err("the challenge was issued to the other ceremony");

        assert_eq!(error.kind(), WebAuthnErrorKind::Challenge);
    }

    #[test]
    fn a_registration_of_a_credential_id_of_another_user_is_refused() {
        let (manager, credentials) = fixture();
        register(&manager, b"the-credential", "Max", 0);
        let key = signing_key();
        let options = manager.create_registration_options("Bob", "Bob");
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, options.challenge(), ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Bob", &request)
            .expect_err("the credential id belongs to Max");

        assert_eq!(error.kind(), WebAuthnErrorKind::Credential);
        assert_eq!(
            credentials
                .find_by_credential_id(b"the-credential")
                .expect("the credential of Max is still stored")
                .user_name(),
            "Max",
            "a registration cannot take over the credential of another user"
        );
    }

    #[test]
    fn a_registration_whose_client_data_belongs_to_an_authentication_is_refused() {
        let (manager, _) = fixture();
        let key = signing_key();
        let options = manager.create_registration_options("Max", "Max");
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(GET, options.challenge(), ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Max", &request)
            .expect_err("the client data belongs to another ceremony");

        assert_eq!(error.kind(), WebAuthnErrorKind::ClientData);
        assert!(error.message().contains("webauthn.create"), "was: {error}");
    }

    #[test]
    fn a_registration_whose_credential_id_is_not_the_one_it_attests_is_refused() {
        let (manager, _) = fixture();
        let key = signing_key();
        let options = manager.create_registration_options("Max", "Max");
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-attested-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"another-credential",
            &client_data_json(CREATE, options.challenge(), ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Max", &request)
            .expect_err("the request and the attestation name different credentials");

        assert_eq!(error.kind(), WebAuthnErrorKind::Credential);
    }

    #[test]
    fn without_an_allowed_origin_no_ceremony_runs() {
        let mut manager = WebAuthnManager::new(relying_party());
        manager.allowed_origins(Vec::<String>::new());
        let options = manager.create_registration_options("Max", "Max");
        let key = signing_key();
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, options.challenge(), ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Max", &request)
            .expect_err("a relying party that allows no origin refuses every ceremony");

        assert_eq!(error.kind(), WebAuthnErrorKind::Configuration);
    }

    #[test]
    fn the_options_of_an_authentication_offer_the_credentials_of_the_user() {
        let (manager, _) = fixture();
        register(&manager, b"the-credential", "Max", 0);

        let options = manager.create_authentication_options("Max");
        let json = serde_json::to_value(&options).expect("the options serialize");

        assert_eq!(json["rpId"], RP_ID);
        assert_eq!(
            json["allowCredentials"][0]["id"],
            crate::webauthn::test_support::to_base64_url(b"the-credential")
        );
    }

    #[test]
    fn an_authentication_that_holds_every_condition_authenticates() {
        let (manager, credentials) = fixture();
        register(&manager, b"the-credential", "Max", 0);
        let submission = authentication(&manager, b"the-credential", 1, None);
        let request = AuthenticationRequest::parse(&submission).expect("the request parses");

        let credential = manager
            .authenticate(&request)
            .expect("the ceremony holds without a user handle");

        assert_eq!(credential.user_name(), "Max");
        assert_eq!(credential.sign_count(), 1);
        assert_eq!(
            credentials
                .find_by_credential_id(b"the-credential")
                .expect("the credential is stored")
                .sign_count(),
            1,
            "the counter of the authenticator is recorded"
        );
    }

    #[test]
    fn an_authentication_of_a_discoverable_credential_reports_its_user() {
        let (manager, _) = fixture();
        register(&manager, b"the-credential", "Max", 0);
        let submission = authentication(&manager, b"the-credential", 1, Some(b"Max"));
        let request = AuthenticationRequest::parse(&submission).expect("the request parses");

        let credential = manager
            .authenticate(&request)
            .expect("the user handle names the user of the credential");

        assert_eq!(credential.user_name(), "Max");
    }

    #[test]
    fn an_authentication_of_another_user_is_refused() {
        let (manager, _) = fixture();
        register(&manager, b"the-credential", "Max", 0);
        let submission = authentication(&manager, b"the-credential", 1, Some(b"Bob"));
        let request = AuthenticationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .authenticate(&request)
            .expect_err("the credential belongs to Max");

        assert_eq!(error.kind(), WebAuthnErrorKind::Credential);
        assert!(error.message().contains("another user"), "was: {error}");
    }

    #[test]
    fn a_tampered_signature_is_refused() {
        let (manager, _) = fixture();
        register(&manager, b"the-credential", "Max", 0);
        let submission = authentication(&manager, b"the-credential", 1, None);
        let mut document: serde_json::Value =
            serde_json::from_slice(&submission).expect("the submission is JSON");
        let mut signature = crate::webauthn::test_support::base64_url(
            document["response"]["signature"]
                .as_str()
                .expect("the signature is a string"),
        );
        signature[10] ^= 0x01;
        document["response"]["signature"] = serde_json::json!(
            crate::webauthn::test_support::to_base64_url(&signature)
        );
        let request = AuthenticationRequest::parse(
            &serde_json::to_vec(&document).expect("the submission serializes"),
        )
        .expect("the request parses");

        let error = manager
            .authenticate(&request)
            .expect_err("the signature does not verify");

        assert_eq!(error.kind(), WebAuthnErrorKind::Signature);
    }

    #[test]
    fn an_authentication_of_a_credential_that_was_never_registered_is_refused() {
        let (manager, _) = fixture();
        register(&manager, b"the-credential", "Max", 0);
        let submission = authentication(&manager, b"another-credential", 1, None);
        let request = AuthenticationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .authenticate(&request)
            .expect_err("no credential of this relying party has that id");

        assert_eq!(error.kind(), WebAuthnErrorKind::Credential);
    }

    #[test]
    fn an_authentication_of_another_origin_is_refused() {
        let (manager, _) = fixture();
        register(&manager, b"the-credential", "Max", 0);
        let key = signing_key();
        let options = manager.create_authentication_options("Max");
        let client_data = client_data_json(GET, options.challenge(), "https://evil.example.org", false);
        let authenticator_data = authenticator_data(RP_ID, USER_PRESENT, 1, None);
        let mut message = authenticator_data.clone();
        message.extend_from_slice(&sha256(&client_data));
        let submission = authentication_submission(
            b"the-credential",
            &client_data,
            &authenticator_data,
            &sign(&key, &message),
            None,
        );
        let request = AuthenticationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .authenticate(&request)
            .expect_err("the ceremony did not run on the origin of the relying party");

        assert_eq!(error.kind(), WebAuthnErrorKind::ClientData);
        assert!(error.message().contains("evil.example.org"), "was: {error}");
    }

    #[test]
    fn an_authentication_of_another_relying_party_is_refused() {
        let (manager, _) = fixture();
        register(&manager, b"the-credential", "Max", 0);
        let key = signing_key();
        let options = manager.create_authentication_options("Max");
        let client_data = client_data_json(GET, options.challenge(), ORIGIN, false);
        let authenticator_data = authenticator_data("evil.example.org", USER_PRESENT, 1, None);
        let mut message = authenticator_data.clone();
        message.extend_from_slice(&sha256(&client_data));
        let submission = authentication_submission(
            b"the-credential",
            &client_data,
            &authenticator_data,
            &sign(&key, &message),
            None,
        );
        let request = AuthenticationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .authenticate(&request)
            .expect_err("the assertion belongs to another relying party");

        assert_eq!(error.kind(), WebAuthnErrorKind::AuthenticatorData);
    }

    #[test]
    fn an_authentication_whose_sign_count_went_backwards_is_refused() {
        let (manager, _) = fixture();
        register(&manager, b"the-credential", "Max", 7);
        let submission = authentication(&manager, b"the-credential", 3, None);
        let request = AuthenticationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .authenticate(&request)
            .expect_err("a counter that went backwards is the signal of a clone");

        assert_eq!(error.kind(), WebAuthnErrorKind::Credential);
        assert!(error.message().contains("clone"), "was: {error}");
    }

    #[test]
    fn an_authentication_that_answers_a_replayed_challenge_is_refused() {
        let (manager, _) = fixture();
        register(&manager, b"the-credential", "Max", 0);
        let submission = authentication(&manager, b"the-credential", 1, None);
        let request = AuthenticationRequest::parse(&submission).expect("the request parses");

        manager
            .authenticate(&request)
            .expect("the first answer of the challenge holds");
        let error = manager
            .authenticate(&request)
            .expect_err("the challenge was used up by the first answer");

        assert_eq!(error.kind(), WebAuthnErrorKind::Challenge);
    }

    #[test]
    fn an_authentication_whose_client_data_belongs_to_a_registration_is_refused() {
        let (manager, _) = fixture();
        register(&manager, b"the-credential", "Max", 0);
        let key = signing_key();
        let options = manager.create_authentication_options("Max");
        let client_data = client_data_json(CREATE, options.challenge(), ORIGIN, false);
        let authenticator_data = authenticator_data(RP_ID, USER_PRESENT, 1, None);
        let mut message = authenticator_data.clone();
        message.extend_from_slice(&sha256(&client_data));
        let submission = authentication_submission(
            b"the-credential",
            &client_data,
            &authenticator_data,
            &sign(&key, &message),
            None,
        );
        let request = AuthenticationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .authenticate(&request)
            .expect_err("the client data belongs to a registration");

        assert_eq!(error.kind(), WebAuthnErrorKind::ClientData);
        assert!(error.message().contains("webauthn.get"), "was: {error}");
    }

    #[test]
    fn a_ceremony_whose_challenge_expired_is_refused() {
        let (mut manager, _) = fixture();
        manager.challenge_ttl(Duration::seconds(-1));
        let key = signing_key();
        let options = manager.create_registration_options("Max", "Max");
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, options.challenge(), ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Max", &request)
            .expect_err("the challenge outlived its window");

        assert_eq!(error.kind(), WebAuthnErrorKind::Challenge);
    }

    #[test]
    fn a_registration_of_a_user_verification_that_was_not_performed_is_refused() {
        let (mut manager, _) = fixture();
        manager.require_user_verification(true);
        let key = signing_key();
        let options = manager.create_registration_options("Max", "Max");
        let cose_key = cose_key_of(&key);
        let authenticator_data = authenticator_data(
            RP_ID,
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            0,
            Some((b"the-credential", &cose_key)),
        );
        let submission = registration_submission(
            b"the-credential",
            &client_data_json(CREATE, options.challenge(), ORIGIN, false),
            &attestation_object("none", &[], &authenticator_data),
        );
        let request = RegistrationRequest::parse(&submission).expect("the request parses");

        let error = manager
            .register("Max", &request)
            .expect_err("the ceremony requires a verified user");

        assert_eq!(error.kind(), WebAuthnErrorKind::AuthenticatorData);
        assert!(error.message().contains("verified user"), "was: {error}");
    }
}
