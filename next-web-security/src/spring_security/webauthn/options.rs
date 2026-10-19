//! The options a relying party hands to the browser to start a ceremony.
//!
//! The browser reads a JSON document, and every byte string in it travels as
//! base64url — a `Vec<u8>` field is serialized by the helpers below, so the shape
//! of the document follows the specification field by field rather than the
//! shape of the structs.

use serde::{Deserialize, Serialize};

/// The relying party a ceremony runs for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelyingParty {
    id: String,
    name: String,
}

impl RelyingParty {
    /// Creates the relying party with its identifier and its display name.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }

    /// Returns the identifier the authenticator binds the credential to.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the name the browser shows to the user.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// The user a credential is registered for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKeyCredentialUserEntity {
    #[serde(with = "crate::webauthn::base64url")]
    id: Vec<u8>,
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

impl PublicKeyCredentialUserEntity {
    /// Creates the user with the identifier the authenticator stores.
    pub fn new(
        id: impl Into<Vec<u8>>,
        name: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            display_name: display_name.into(),
        }
    }

    /// Returns the identifier of the user.
    pub fn id(&self) -> &[u8] {
        &self.id
    }

    /// Returns the name of the user.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A credential the relying party already knows about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKeyCredentialDescriptor {
    #[serde(rename = "type")]
    kind: String,
    #[serde(with = "crate::webauthn::base64url")]
    id: Vec<u8>,
}

impl PublicKeyCredentialDescriptor {
    /// The type of a credential the browser can use.
    pub const PUBLIC_KEY: &'static str = "public-key";

    /// Creates the descriptor of a credential.
    pub fn new(id: impl Into<Vec<u8>>) -> Self {
        Self {
            kind: Self::PUBLIC_KEY.to_string(),
            id: id.into(),
        }
    }

    /// Returns the identifier of the credential.
    pub fn id(&self) -> &[u8] {
        &self.id
    }
}

/// A signature algorithm the relying party accepts for a credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKeyCredentialParameters {
    #[serde(rename = "type")]
    kind: String,
    alg: i64,
}

impl PublicKeyCredentialParameters {
    /// Creates a parameter for the given COSE algorithm.
    pub fn new(alg: i64) -> Self {
        Self {
            kind: PublicKeyCredentialDescriptor::PUBLIC_KEY.to_string(),
            alg,
        }
    }

    /// Returns the COSE identifier of the algorithm.
    pub fn alg(&self) -> i64 {
        self.alg
    }
}

/// The options of a registration ceremony.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKeyCredentialCreationOptions {
    #[serde(with = "crate::webauthn::base64url")]
    challenge: Vec<u8>,
    rp: RelyingParty,
    user: PublicKeyCredentialUserEntity,
    #[serde(rename = "pubKeyCredParams")]
    pub_key_cred_params: Vec<PublicKeyCredentialParameters>,
    #[serde(rename = "excludeCredentials", skip_serializing_if = "Option::is_none")]
    exclude_credentials: Option<Vec<PublicKeyCredentialDescriptor>>,
    #[serde(rename = "userVerification", skip_serializing_if = "Option::is_none")]
    user_verification: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attestation: Option<String>,
}

impl PublicKeyCredentialCreationOptions {
    /// Creates the options of a registration ceremony.
    pub fn new(
        challenge: impl Into<Vec<u8>>,
        rp: RelyingParty,
        user: PublicKeyCredentialUserEntity,
    ) -> Self {
        Self {
            challenge: challenge.into(),
            rp,
            user,
            // ES256 and RS256 are the two algorithms every authenticator has to
            // support, -7 and -257 in the numbering of COSE.
            pub_key_cred_params: vec![
                PublicKeyCredentialParameters::new(-7),
                PublicKeyCredentialParameters::new(-257),
            ],
            exclude_credentials: None,
            user_verification: None,
            attestation: None,
        }
    }

    /// Sets the credentials the browser must not register again.
    #[must_use]
    pub fn exclude_credentials(mut self, credentials: Vec<PublicKeyCredentialDescriptor>) -> Self {
        self.exclude_credentials = Some(credentials);
        self
    }

    /// Sets the attestation conveyance preference.
    #[must_use]
    pub fn attestation(mut self, attestation: impl Into<String>) -> Self {
        self.attestation = Some(attestation.into());
        self
    }

    /// Sets the user verification the ceremony asks the browser for.
    ///
    /// A ceremony that enforces a verified user has to ask for one: an
    /// authenticator that was only told `preferred` may take part without
    /// verifying anybody, and the ceremony would then refuse it.
    #[must_use]
    pub fn user_verification(mut self, user_verification: impl Into<String>) -> Self {
        self.user_verification = Some(user_verification.into());
        self
    }

    /// Returns the challenge the ceremony answers.
    pub fn challenge(&self) -> &[u8] {
        &self.challenge
    }

    /// Returns the relying party of the ceremony.
    pub fn rp(&self) -> &RelyingParty {
        &self.rp
    }

    /// Returns the user the credential is registered for.
    pub fn user(&self) -> &PublicKeyCredentialUserEntity {
        &self.user
    }
}

/// The options of an authentication ceremony.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKeyCredentialRequestOptions {
    #[serde(with = "crate::webauthn::base64url")]
    challenge: Vec<u8>,
    #[serde(rename = "rpId", skip_serializing_if = "Option::is_none")]
    rp_id: Option<String>,
    #[serde(rename = "allowCredentials", skip_serializing_if = "Option::is_none")]
    allow_credentials: Option<Vec<PublicKeyCredentialDescriptor>>,
    #[serde(rename = "userVerification", skip_serializing_if = "Option::is_none")]
    user_verification: Option<String>,
}

impl PublicKeyCredentialRequestOptions {
    /// Creates the options of an authentication ceremony.
    pub fn new(challenge: impl Into<Vec<u8>>) -> Self {
        Self {
            challenge: challenge.into(),
            rp_id: None,
            allow_credentials: None,
            user_verification: None,
        }
    }

    /// Restricts the ceremony to the credentials of one relying party.
    #[must_use]
    pub fn rp_id(mut self, rp_id: impl Into<String>) -> Self {
        self.rp_id = Some(rp_id.into());
        self
    }

    /// Restricts the ceremony to the given credentials.
    #[must_use]
    pub fn allow_credentials(mut self, credentials: Vec<PublicKeyCredentialDescriptor>) -> Self {
        self.allow_credentials = Some(credentials);
        self
    }

    /// Sets the user verification the ceremony asks the browser for.
    ///
    /// The same rule as for a registration: an authenticator that was not told
    /// that verification is required may take part without verifying anybody.
    #[must_use]
    pub fn user_verification(mut self, user_verification: impl Into<String>) -> Self {
        self.user_verification = Some(user_verification.into());
        self
    }

    /// Returns the challenge the ceremony answers.
    pub fn challenge(&self) -> &[u8] {
        &self.challenge
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a registration ceremony for `Max`.
    fn creation_options() -> PublicKeyCredentialCreationOptions {
        PublicKeyCredentialCreationOptions::new(
            b"the-challenge".to_vec(),
            RelyingParty::new("sp.example.org", "The application"),
            PublicKeyCredentialUserEntity::new(b"the-user".to_vec(), "max", "Max"),
        )
    }

    #[test]
    fn the_options_of_a_registration_carry_the_names_the_browser_reads() {
        let json = serde_json::to_value(creation_options()).expect("the options serialize");
        let object = json.as_object().expect("the options are an object");

        assert_eq!(
            object.get("challenge").and_then(|value| value.as_str()),
            Some("dGhlLWNoYWxsZW5nZQ"),
            "the challenge is base64url, without padding"
        );
        assert_eq!(
            object
                .get("user")
                .and_then(|user| user.get("displayName"))
                .and_then(|value| value.as_str()),
            Some("Max")
        );
        assert!(
            object.contains_key("pubKeyCredParams"),
            "the algorithms are named the way the browser reads them"
        );
        assert!(
            object.get("rp").and_then(|rp| rp.get("id")).is_some(),
            "the relying party travels with its id"
        );
    }

    #[test]
    fn the_algorithms_offered_are_the_two_every_authenticator_supports() {
        let options = creation_options();
        let json = serde_json::to_value(&options).expect("the options serialize");
        let algorithms = json["pubKeyCredParams"]
            .as_array()
            .expect("the algorithms are a list")
            .iter()
            .map(|parameter| parameter["alg"].as_i64().expect("an algorithm number"))
            .collect::<Vec<_>>();

        assert_eq!(algorithms, vec![-7, -257], "ES256 and RS256");
    }

    #[test]
    fn the_options_of_an_authentication_carry_its_restrictions() {
        let options = PublicKeyCredentialRequestOptions::new(b"the-challenge".to_vec())
            .rp_id("sp.example.org")
            .allow_credentials(vec![PublicKeyCredentialDescriptor::new(
                b"the-credential".to_vec(),
            )]);

        let json = serde_json::to_value(options).expect("the options serialize");

        assert_eq!(json["rpId"], "sp.example.org");
        assert_eq!(json["allowCredentials"][0]["type"], "public-key");
        assert_eq!(
            json["allowCredentials"][0]["id"],
            "dGhlLWNyZWRlbnRpYWw",
            "the credential id is base64url"
        );
    }

    #[test]
    fn the_options_that_hold_no_restriction_do_not_carry_the_field() {
        let json = serde_json::to_value(PublicKeyCredentialRequestOptions::new(
            b"the-challenge".to_vec(),
        ))
        .expect("the options serialize");

        assert_eq!(json.get("rpId"), None);
        assert_eq!(json.get("allowCredentials"), None);
    }

    #[test]
    fn the_user_verification_a_ceremony_asks_for_is_what_the_browser_reads() {
        let creation = creation_options().user_verification("required");
        let json = serde_json::to_value(&creation).expect("the options serialize");

        assert_eq!(json["userVerification"], "required");

        let request = PublicKeyCredentialRequestOptions::new(b"the-challenge".to_vec())
            .user_verification("required");
        let json = serde_json::to_value(&request).expect("the options serialize");

        assert_eq!(json["userVerification"], "required");
    }

    #[test]
    fn a_user_identifier_survives_the_round_trip() {
        let user = PublicKeyCredentialUserEntity::new(b"the-user".to_vec(), "max", "Max");
        let json = serde_json::to_string(&user).expect("the user serializes");
        let read: PublicKeyCredentialUserEntity =
            serde_json::from_str(&json).expect("the user parses");

        assert_eq!(read.id(), b"the-user");
        assert_eq!(read.name(), "max");
    }
}
