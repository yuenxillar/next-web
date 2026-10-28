//! The requests a browser submits to finish a ceremony.
//!
//! The browser answers with a JSON document whose byte strings are base64url:
//! the credential id, the client data it signed, and — depending on the
//! ceremony — the attestation object of a registration or the authenticator
//! data and the signature of an authentication.

use serde::Deserialize;

use super::{
    base64url,
    error::{WebAuthnError, WebAuthnErrorKind},
};

/// The type of a credential the browser can submit.
const PUBLIC_KEY: &str = "public-key";

/// The submission of a registration ceremony.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RegistrationRequest {
    #[serde(with = "base64url")]
    id: Vec<u8>,
    #[serde(rename = "rawId", with = "base64url")]
    raw_id: Vec<u8>,
    #[serde(rename = "type")]
    kind: String,
    response: RegistrationResponse,
}

/// The response of a registration ceremony.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RegistrationResponse {
    #[serde(rename = "clientDataJSON", with = "base64url")]
    client_data_json: Vec<u8>,
    #[serde(rename = "attestationObject", with = "base64url")]
    attestation_object: Vec<u8>,
    #[serde(default)]
    transports: Vec<String>,
}

impl RegistrationRequest {
    /// Parses the JSON document the browser submits.
    pub fn parse(raw: &[u8]) -> Result<Self, WebAuthnError> {
        let request: Self = serde_json::from_slice(raw).map_err(|error| {
            WebAuthnError::new(
                WebAuthnErrorKind::Request,
                format!("the registration request is not the JSON of a ceremony: {error}"),
            )
        })?;
        request.check()?;

        Ok(request)
    }

    /// Checks the parts of the request that hold whatever the ceremony does.
    fn check(&self) -> Result<(), WebAuthnError> {
        if self.kind != PUBLIC_KEY {
            return Err(WebAuthnError::new(
                WebAuthnErrorKind::Request,
                format!(
                    "the registration request is of type {}, and a ceremony submits {PUBLIC_KEY}",
                    self.kind
                ),
            ));
        }
        if self.id != self.raw_id {
            return Err(WebAuthnError::new(
                WebAuthnErrorKind::Request,
                "the registration request names the credential with an id that is not its raw id",
            ));
        }

        Ok(())
    }

    /// Returns the identifier the authenticator chose for the credential.
    pub fn credential_id(&self) -> &[u8] {
        &self.raw_id
    }

    /// Returns the client data the browser signed.
    pub fn client_data_json(&self) -> &[u8] {
        &self.response.client_data_json
    }

    /// Returns the attestation object of the registration.
    pub fn attestation_object(&self) -> &[u8] {
        &self.response.attestation_object
    }

    /// Returns the transports the authenticator reports, which a later
    /// authentication may hint at.
    pub fn transports(&self) -> &[String] {
        &self.response.transports
    }
}

/// The submission of an authentication ceremony.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AuthenticationRequest {
    #[serde(with = "base64url")]
    id: Vec<u8>,
    #[serde(rename = "rawId", with = "base64url")]
    raw_id: Vec<u8>,
    #[serde(rename = "type")]
    kind: String,
    response: AuthenticationResponse,
}

/// The response of an authentication ceremony.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AuthenticationResponse {
    #[serde(rename = "clientDataJSON", with = "base64url")]
    client_data_json: Vec<u8>,
    #[serde(rename = "authenticatorData", with = "base64url")]
    authenticator_data: Vec<u8>,
    #[serde(with = "base64url")]
    signature: Vec<u8>,
    #[serde(rename = "userHandle", default, with = "base64url_option")]
    user_handle: Option<Vec<u8>>,
}

impl AuthenticationRequest {
    /// Parses the JSON document the browser submits.
    pub fn parse(raw: &[u8]) -> Result<Self, WebAuthnError> {
        let request: Self = serde_json::from_slice(raw).map_err(|error| {
            WebAuthnError::new(
                WebAuthnErrorKind::Request,
                format!("the authentication request is not the JSON of a ceremony: {error}"),
            )
        })?;
        request.check()?;

        Ok(request)
    }

    /// Checks the parts of the request that hold whatever the ceremony does.
    fn check(&self) -> Result<(), WebAuthnError> {
        if self.kind != PUBLIC_KEY {
            return Err(WebAuthnError::new(
                WebAuthnErrorKind::Request,
                format!(
                    "the authentication request is of type {}, and a ceremony submits {PUBLIC_KEY}",
                    self.kind
                ),
            ));
        }
        if self.id != self.raw_id {
            return Err(WebAuthnError::new(
                WebAuthnErrorKind::Request,
                "the authentication request names the credential with an id that is not its raw id",
            ));
        }

        Ok(())
    }

    /// Returns the identifier of the credential that was used.
    pub fn credential_id(&self) -> &[u8] {
        &self.raw_id
    }

    /// Returns the client data the browser signed.
    pub fn client_data_json(&self) -> &[u8] {
        &self.response.client_data_json
    }

    /// Returns the authenticator data of the ceremony, which the signature covers.
    pub fn authenticator_data(&self) -> &[u8] {
        &self.response.authenticator_data
    }

    /// Returns the signature over the ceremony.
    pub fn signature(&self) -> &[u8] {
        &self.response.signature
    }

    /// Returns the user handle the browser sent, when the credential is
    /// discoverable and the browser knows which user it belongs to.
    pub fn user_handle(&self) -> Option<&[u8]> {
        self.response.user_handle.as_deref()
    }
}

/// Reads an optional base64url byte string, which the browser sends as `null`
/// when there is none.
mod base64url_option {
    use serde::{Deserialize, Deserializer};

    /// Reads the byte string back into bytes when it is present.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<String>::deserialize(deserializer)?;

        match value {
            Some(value) => super::base64url::decode(&value)
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::webauthn::test_support::to_base64_url;

    /// Returns the JSON a browser submits to register a credential.
    fn registration_json() -> Vec<u8> {
        serde_json::to_vec(&json!({
            "id": to_base64_url(b"the-credential"),
            "rawId": to_base64_url(b"the-credential"),
            "type": "public-key",
            "response": {
                "clientDataJSON": to_base64_url(b"the-client-data"),
                "attestationObject": to_base64_url(b"the-attestation"),
                "transports": ["internal", "hybrid"],
            },
        }))
        .expect("the document serializes")
    }

    /// Returns the JSON a browser submits to authenticate.
    fn authentication_json(user_handle: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "id": to_base64_url(b"the-credential"),
            "rawId": to_base64_url(b"the-credential"),
            "type": "public-key",
            "response": {
                "clientDataJSON": to_base64_url(b"the-client-data"),
                "authenticatorData": to_base64_url(b"the-authenticator-data"),
                "signature": to_base64_url(b"the-signature"),
                "userHandle": user_handle,
            },
        }))
        .expect("the document serializes")
    }

    #[test]
    fn the_registration_request_of_a_browser_is_read() {
        let request = RegistrationRequest::parse(&registration_json())
            .expect("the registration request parses");

        assert_eq!(request.credential_id(), b"the-credential");
        assert_eq!(request.client_data_json(), b"the-client-data");
        assert_eq!(request.attestation_object(), b"the-attestation");
        assert_eq!(request.transports(), ["internal", "hybrid"]);
    }

    #[test]
    fn a_registration_request_of_another_type_is_refused() {
        let mut document: serde_json::Value =
            serde_json::from_slice(&registration_json()).expect("the document is JSON");
        document["type"] = json!("password");

        let error = RegistrationRequest::parse(
            &serde_json::to_vec(&document).expect("the document serializes"),
        )
        .expect_err("a password is not a passkey");

        assert_eq!(error.kind(), WebAuthnErrorKind::Request);
        assert!(error.message().contains("public-key"), "was: {error}");
    }

    #[test]
    fn a_registration_request_whose_id_is_not_its_raw_id_is_refused() {
        let mut document: serde_json::Value =
            serde_json::from_slice(&registration_json()).expect("the document is JSON");
        document["id"] = json!(to_base64_url(b"another-credential"));

        let error = RegistrationRequest::parse(
            &serde_json::to_vec(&document).expect("the document serializes"),
        )
        .expect_err("the two names of the credential have to agree");

        assert_eq!(error.kind(), WebAuthnErrorKind::Request);
    }

    #[test]
    fn a_registration_request_that_is_not_json_is_refused() {
        let error =
            RegistrationRequest::parse(b"not the JSON of a ceremony").expect_err("this is not JSON");

        assert_eq!(error.kind(), WebAuthnErrorKind::Request);
    }

    #[test]
    fn a_registration_request_whose_byte_string_is_padded_is_refused() {
        let mut document: serde_json::Value =
            serde_json::from_slice(&registration_json()).expect("the document is JSON");
        document["rawId"] = json!("dGhlLWNyZWRlbnRpYWw=");
        document["id"] = json!("dGhlLWNyZWRlbnRpYWw=");

        let error = RegistrationRequest::parse(
            &serde_json::to_vec(&document).expect("the document serializes"),
        )
        .expect_err("a padded byte string is not the form the browser writes");

        assert_eq!(error.kind(), WebAuthnErrorKind::Request);
    }

    #[test]
    fn the_authentication_request_of_a_browser_is_read() {
        let request = AuthenticationRequest::parse(&authentication_json(json!(null)))
            .expect("the authentication request parses");

        assert_eq!(request.credential_id(), b"the-credential");
        assert_eq!(request.client_data_json(), b"the-client-data");
        assert_eq!(request.authenticator_data(), b"the-authenticator-data");
        assert_eq!(request.signature(), b"the-signature");
        assert_eq!(request.user_handle(), None, "no user handle was sent");
    }

    #[test]
    fn the_user_handle_of_a_discoverable_credential_is_read() {
        let request = AuthenticationRequest::parse(&authentication_json(json!(to_base64_url(
            b"max"
        ))))
        .expect("the authentication request parses");

        assert_eq!(request.user_handle(), Some(b"max".as_slice()));
    }

    #[test]
    fn an_authentication_request_of_another_type_is_refused() {
        let mut document: serde_json::Value =
            serde_json::from_slice(&authentication_json(json!(null))).expect("the document is JSON");
        document["type"] = json!("password");

        let error = AuthenticationRequest::parse(
            &serde_json::to_vec(&document).expect("the document serializes"),
        )
        .expect_err("a password is not a passkey");

        assert_eq!(error.kind(), WebAuthnErrorKind::Request);
    }

    #[test]
    fn an_authentication_request_without_a_signature_is_refused() {
        let mut document: serde_json::Value =
            serde_json::from_slice(&authentication_json(json!(null))).expect("the document is JSON");
        document["response"]
            .as_object_mut()
            .expect("the response is an object")
            .remove("signature");

        let error = AuthenticationRequest::parse(
            &serde_json::to_vec(&document).expect("the document serializes"),
        )
        .expect_err("a ceremony without a signature proves nothing");

        assert_eq!(error.kind(), WebAuthnErrorKind::Request);
    }
}
