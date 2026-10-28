//! The client data of a Web Authentication ceremony.
//!
//! A ceremony carries the data of the client twice: once signed, as the raw bytes
//! of a JSON document, and once as that document encoded with base64url. The
//! authenticator signs the hash of the raw bytes, which is why the raw bytes are
//! what this module parses — decoding and re-encoding the JSON would change them.

use std::{error::Error, fmt};

use base64::Engine;
use serde::Deserialize;

/// The type of a registration ceremony.
pub const CREATE: &str = "webauthn.create";

/// The type of an authentication ceremony.
pub const GET: &str = "webauthn.get";

/// The client data a ceremony carries.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ClientData {
    #[serde(rename = "type")]
    kind: String,
    challenge: String,
    origin: String,
    #[serde(rename = "crossOrigin", default)]
    cross_origin: bool,
}

impl ClientData {
    /// Parses the raw bytes the authenticator signed.
    pub fn parse(raw: &[u8]) -> Result<Self, ClientDataError> {
        serde_json::from_slice(raw)
            .map_err(|error| ClientDataError::new(format!("the client data is not JSON: {error}")))
    }

    /// Parses the client data the browser submits, which is base64url encoded.
    pub fn parse_base64_url(value: &str) -> Result<Self, ClientDataError> {
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(value)
            .or_else(|_| base64::engine::general_purpose::STANDARD.decode(value))
            .map_err(|error| {
                ClientDataError::new(format!("the client data is not base64url: {error}"))
            })?;

        Self::parse(&raw)
    }

    /// Returns the type of the ceremony the data belongs to.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns the challenge the client answered.
    pub fn challenge(&self) -> &str {
        &self.challenge
    }

    /// Returns the challenge the client answered, as the bytes the relying
    /// party issued.
    ///
    /// The challenge travels as a base64url string, so it is decoded before it
    /// is looked up: the store holds the bytes it handed out, not one spelling
    /// of them. Only the canonical spelling is accepted, so that one challenge
    /// cannot be written in two ways.
    pub fn challenge_bytes(&self) -> Result<Vec<u8>, ClientDataError> {
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let raw = engine.decode(&self.challenge).map_err(|error| {
            ClientDataError::new(format!(
                "the challenge of the client data is not base64url: {error}"
            ))
        })?;
        if engine.encode(&raw) != self.challenge {
            return Err(ClientDataError::new(
                "the challenge of the client data is not the canonical base64url form of the \
                 bytes it decodes to",
            ));
        }

        Ok(raw)
    }

    /// Returns the origin of the page that asked for the ceremony.
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// Returns whether the requesting context was a cross-origin frame, which the
    /// relying party has to refuse.
    pub fn is_cross_origin(&self) -> bool {
        self.cross_origin
    }

    /// Checks the data against what the relying party asked for.
    ///
    /// The checks here are the ones the data itself can answer: the ceremony it
    /// belongs to, and the page it ran on — which has to be one of the origins
    /// the relying party serves. The challenge is answered by the store that
    /// issued it: a string comparison against a string the caller already holds
    /// would accept a challenge this relying party never issued.
    pub fn validate(
        &self,
        expected_kind: &str,
        allowed_origins: &[String],
    ) -> Result<(), ClientDataError> {
        if self.is_cross_origin() {
            return Err(ClientDataError::new(
                "the ceremony was requested by a cross-origin frame",
            ));
        }
        if self.kind != expected_kind {
            return Err(ClientDataError::new(format!(
                "the client data belongs to {}, not to {expected_kind}",
                self.kind
            )));
        }
        if !allowed_origins.iter().any(|origin| origin == &self.origin) {
            return Err(ClientDataError::new(format!(
                "the client data comes from {}, which is not one of the origins the relying party \
                 serves ({})",
                self.origin,
                allowed_origins.join(", ")
            )));
        }

        Ok(())
    }
}

/// The error of reading or checking the client data of a ceremony.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientDataError {
    message: String,
}

impl ClientDataError {
    /// Creates the error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the message of the error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ClientDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ClientDataError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the origins the tests allow a ceremony on.
    fn origins() -> Vec<String> {
        vec!["https://sp.example.org".to_string()]
    }

    /// Returns the raw JSON of a registration ceremony.
    fn create_data(challenge: &str, origin: &str) -> String {
        format!(
            r#"{{"type":"webauthn.create","challenge":"{challenge}","origin":"{origin}","crossOrigin":false}}"#
        )
    }

    #[test]
    fn the_client_data_of_a_ceremony_is_parsed() {
        let data = ClientData::parse(create_data("the-challenge", "https://sp.example.org").as_bytes())
            .expect("the client data parses");

        assert_eq!(data.kind(), CREATE);
        assert_eq!(data.challenge(), "the-challenge");
        assert_eq!(data.origin(), "https://sp.example.org");
        assert!(!data.is_cross_origin());
    }

    #[test]
    fn the_client_data_of_a_ceremony_is_read_from_its_base64url_form() {
        use base64::Engine;

        let raw = create_data("the-challenge", "https://sp.example.org");
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw.as_bytes());

        let data = ClientData::parse_base64_url(&encoded).expect("the client data parses");

        assert_eq!(data.challenge(), "the-challenge");
    }

    #[test]
    fn data_of_a_ceremony_that_holds_every_condition_is_accepted() {
        let data = ClientData::parse(create_data("the-challenge", "https://sp.example.org").as_bytes())
            .expect("the client data parses");

        data.validate(CREATE, &origins())
            .expect("the data belongs to the ceremony");
    }

    #[test]
    fn data_of_another_ceremony_is_refused() {
        let data = ClientData::parse(create_data("the-challenge", "https://sp.example.org").as_bytes())
            .expect("the client data parses");

        let error = data
            .validate(GET, &origins())
            .expect_err("a registration is not an authentication");

        assert!(error.message().contains("webauthn.get"), "was: {error}");
    }

    #[test]
    fn the_challenge_of_the_client_data_is_read_back_as_the_bytes_that_were_issued() {
        use base64::Engine;

        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"the-challenge");
        let data = ClientData::parse(create_data(&challenge, "https://sp.example.org").as_bytes())
            .expect("the client data parses");

        assert_eq!(
            data.challenge_bytes()
                .expect("the challenge decodes to the bytes it was issued as"),
            b"the-challenge"
        );
    }

    #[test]
    fn a_challenge_that_is_not_canonical_base64url_is_refused() {
        let data = ClientData::parse(
            create_data("dGhlLWNoYWxsZW5nZQ==", "https://sp.example.org").as_bytes(),
        )
        .expect("the client data parses");

        let error = data
            .challenge_bytes()
            .expect_err("a padded challenge is not the form the relying party issued");

        assert!(error.message().contains("base64url"), "was: {error}");
    }

    #[test]
    fn data_of_another_origin_is_refused() {
        let data = ClientData::parse(create_data("the-challenge", "https://evil.example.org").as_bytes())
            .expect("the client data parses");

        let error = data
            .validate(CREATE, &origins())
            .expect_err("the ceremony did not run on the origin of the relying party");

        assert!(error.message().contains("evil.example.org"), "was: {error}");
    }

    #[test]
    fn data_of_a_cross_origin_frame_is_refused() {
        let data = ClientData::parse(
            br#"{"type":"webauthn.create","challenge":"the-challenge","origin":"https://sp.example.org","crossOrigin":true}"#,
        )
        .expect("the client data parses");

        let error = data
            .validate(CREATE, &origins())
            .expect_err("a cross-origin frame may not run the ceremony");

        assert!(error.message().contains("cross-origin"), "was: {error}");
    }

    #[test]
    fn data_that_is_not_json_is_refused() {
        assert!(ClientData::parse(b"not json").is_err());
    }
}
