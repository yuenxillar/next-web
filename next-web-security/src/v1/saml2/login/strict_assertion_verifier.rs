use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Duration, Utc};

use crate::saml2::{
    error::{Saml2Error, Saml2ErrorKind},
    login::{AssertionVerifier, Saml2Response},
    registration::RelyingPartyRegistration,
};

/// The status a response has to report to be considered at all.
const STATUS_SUCCESS: &str = "urn:oasis:names:tc:SAML:2.0:status:Success";

/// Verifies that the signature of a response covers what the response claims.
///
/// This is the seam that is still open: the port has no XML signature
/// implementation, so an application supplies one — over `samael`, `xmlsec`, or
/// whatever else it trusts — and everything above it works unchanged.
pub trait SignatureVerifier: Send + Sync {
    /// Returns `Ok(())` when the signature of the response is valid for the key
    /// the registration holds.
    fn verify(
        &self,
        registration: &RelyingPartyRegistration,
        response: &Saml2Response,
    ) -> Result<(), Saml2Error>;
}

/// The requests this application started, so a response can be tied to one.
pub trait Saml2AuthenticationRequestRepository: Send + Sync {
    /// Remembers a request until it expires.
    fn save(&self, registration_id: &str, request_id: &str, expires_at: DateTime<Utc>);

    /// Returns the registration of the request and forgets it, so a response
    /// can only answer it once.
    fn consume(&self, request_id: &str) -> Option<String>;
}

/// A [`Saml2AuthenticationRequestRepository`] that holds the requests in memory.
#[derive(Debug, Default)]
pub struct InMemorySaml2AuthenticationRequestRepository {
    requests: RwLock<HashMap<String, (String, DateTime<Utc>)>>,
}

impl Saml2AuthenticationRequestRepository for InMemorySaml2AuthenticationRequestRepository {
    fn save(&self, registration_id: &str, request_id: &str, expires_at: DateTime<Utc>) {
        let mut requests = self
            .requests
            .write()
            .expect("the request repository lock is poisoned");
        requests.retain(|_, (_, expiry)| *expiry > Utc::now());
        requests.insert(request_id.to_owned(), (registration_id.to_owned(), expires_at));
    }

    fn consume(&self, request_id: &str) -> Option<String> {
        let mut requests = self
            .requests
            .write()
            .expect("the request repository lock is poisoned");
        let (registration_id, expires_at) = requests.remove(request_id)?;

        (expires_at > Utc::now()).then_some(registration_id)
    }
}

/// Remembers the assertions that were already used, so none is used twice.
///
/// An assertion is replayable until it expires: the asserting party signed it
/// once, and nothing in the document stops a second submission.
pub trait AssertionReplayCache: Send + Sync {
    /// Records the assertion and returns `false` when it was already recorded.
    fn register(&self, assertion_id: &str, expires_at: DateTime<Utc>) -> bool;
}

/// An [`AssertionReplayCache`] that holds the assertions in memory.
#[derive(Debug, Default)]
pub struct InMemoryAssertionReplayCache {
    seen: RwLock<HashMap<String, DateTime<Utc>>>,
}

impl AssertionReplayCache for InMemoryAssertionReplayCache {
    fn register(&self, assertion_id: &str, expires_at: DateTime<Utc>) -> bool {
        let mut seen = self
            .seen
            .write()
            .expect("the replay cache lock is poisoned");
        let now = Utc::now();
        seen.retain(|_, expiry| *expiry > now);

        if seen.contains_key(assertion_id) {
            return false;
        }
        seen.insert(assertion_id.to_owned(), expires_at);

        true
    }
}

/// Believes a response only when its signature is verified and every condition
/// of the profile holds.
///
/// The verifier is the whole check, in order: the signature, the status the
/// asserting party reports, the two identities (who asserts, who it is for), the
/// endpoint the assertion may be delivered to, the window of the assertion, the
/// request it answers, and whether it was used before.
pub struct StrictAssertionVerifier {
    signature: Arc<dyn SignatureVerifier>,
    requests: Arc<dyn Saml2AuthenticationRequestRepository>,
    replay: Arc<dyn AssertionReplayCache>,
    clock_skew: Duration,
}

impl StrictAssertionVerifier {
    /// Creates a verifier over a signature implementation and two stores.
    pub fn new(
        signature: Arc<dyn SignatureVerifier>,
        requests: Arc<dyn Saml2AuthenticationRequestRepository>,
        replay: Arc<dyn AssertionReplayCache>,
    ) -> Self {
        Self {
            signature,
            requests,
            replay,
            clock_skew: Duration::zero(),
        }
    }

    /// Tolerates the given number of seconds of clock difference.
    #[must_use]
    pub fn clock_skew(mut self, seconds: i64) -> Self {
        self.clock_skew = Duration::seconds(seconds);
        self
    }
}

impl AssertionVerifier for StrictAssertionVerifier {
    fn verify(
        &self,
        registration: &RelyingPartyRegistration,
        response: &Saml2Response,
    ) -> Result<(), Saml2Error> {
        self.signature.verify(registration, response)?;

        if response.status_code() != Some(STATUS_SUCCESS) {
            return Err(failed("the asserting party did not report success"));
        }

        match (response.issuer(), registration.asserting_party_entity_id()) {
            (Some(issuer), Some(expected)) if issuer == expected => {}
            (Some(_), None) => {
                return Err(failed("the registration names no asserting party"));
            }
            _ => return Err(failed("the response comes from another asserting party")),
        }

        if response.audience() != Some(registration.entity_id()) {
            return Err(failed("the response is meant for another audience"));
        }
        if response.recipient() != Some(registration.assertion_consumer_service_location()) {
            return Err(failed("the response is meant for another endpoint"));
        }

        let now = Utc::now();
        let not_before = instant(response.not_before(), "NotBefore")?;
        let not_on_or_after = instant(response.not_on_or_after(), "NotOnOrAfter")?;
        if now + self.clock_skew < not_before || now - self.clock_skew >= not_on_or_after {
            return Err(failed("the assertion is outside its window"));
        }

        let Some(in_response_to) = response.in_response_to() else {
            return Err(failed("the response answers no request"));
        };
        if self.requests.consume(in_response_to).is_none() {
            return Err(failed("the response answers no request of this application"));
        }

        let Some(assertion_id) = response.assertion_id() else {
            return Err(failed("the response carries no assertion identifier"));
        };
        if !self.replay.register(assertion_id, not_on_or_after) {
            return Err(failed("the assertion was already used"));
        }

        Ok(())
    }
}

/// Parses a SAML instant, naming the field that could not be read.
fn instant(value: Option<&str>, field: &str) -> Result<DateTime<Utc>, Saml2Error> {
    let Some(value) = value else {
        return Err(failed(format!("the assertion carries no {field}")));
    };

    DateTime::parse_from_rfc3339(value)
        .map(|instant| instant.with_timezone(&Utc))
        .map_err(|_| failed(format!("the assertion carries an unreadable {field}")))
}

/// Returns the failure of a check that did not hold.
fn failed(message: impl Into<String>) -> Saml2Error {
    Saml2Error::new(Saml2ErrorKind::VerificationFailed, message)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};

    use super::*;
    use crate::saml2::login::Saml2Response;
    use crate::saml2::registration::RelyingPartyRegistration;
    use crate::saml2::Saml2ErrorKind;

    /// A signature verifier that believes every signature.
    struct AcceptingSignature;

    impl SignatureVerifier for AcceptingSignature {
        fn verify(
            &self,
            _registration: &RelyingPartyRegistration,
            _response: &Saml2Response,
        ) -> Result<(), crate::saml2::Saml2Error> {
            Ok(())
        }
    }

    /// A signature verifier that refuses every signature.
    struct RefusingSignature;

    impl SignatureVerifier for RefusingSignature {
        fn verify(
            &self,
            _registration: &RelyingPartyRegistration,
            _response: &Saml2Response,
        ) -> Result<(), crate::saml2::Saml2Error> {
            Err(crate::saml2::Saml2Error::new(
                Saml2ErrorKind::VerificationFailed,
                "the signature does not cover the response",
            ))
        }
    }

    /// Returns the registration a response is submitted for.
    fn registration() -> RelyingPartyRegistration {
        RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("sp-entity-id")
            .assertion_consumer_service_location("https://sp.example.org/login/saml2/sso/okta")
            .asserting_party_entity_id("idp-entity-id")
            .build()
    }

    /// Returns a response document around the given values.
    fn response_xml(
        assertion_id: &str,
        in_response_to: &str,
        not_before: &str,
        not_on_or_after: &str,
        audience: &str,
        recipient: &str,
        status: &str,
    ) -> String {
        format!(
            r#"<saml2p:Response xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" Destination="{recipient}" InResponseTo="{in_response_to}"><saml2:Issuer>idp-entity-id</saml2:Issuer><saml2p:Status><saml2p:StatusCode Value="{status}"/></saml2p:Status><saml2:Assertion ID="{assertion_id}"><saml2:Issuer>idp-entity-id</saml2:Issuer><ds:Signature xmlns:ds="http://www.w3.org/2000/09/xmldsig#"/><saml2:Subject><saml2:NameID>Max</saml2:NameID><saml2:SubjectConfirmation><saml2:SubjectConfirmationData Recipient="{recipient}"/></saml2:SubjectConfirmation></saml2:Subject><saml2:Conditions NotBefore="{not_before}" NotOnOrAfter="{not_on_or_after}"><saml2:AudienceRestriction><saml2:Audience>{audience}</saml2:Audience></saml2:AudienceRestriction></saml2:Conditions></saml2:Assertion></saml2p:Response>"#
        )
    }

    /// Returns a response that holds every condition, with the given changes
    /// applied on top.
    fn valid_response(assertion_id: &str, in_response_to: &str) -> Saml2Response {
        let not_before = (Utc::now() - Duration::minutes(1)).to_rfc3339();
        let not_on_or_after = (Utc::now() + Duration::minutes(5)).to_rfc3339();

        Saml2Response::parse(&response_xml(
            assertion_id,
            in_response_to,
            &not_before,
            &not_on_or_after,
            "sp-entity-id",
            "https://sp.example.org/login/saml2/sso/okta",
            "urn:oasis:names:tc:SAML:2.0:status:Success",
        ))
        .expect("the response parses")
    }

    /// Returns a strict verifier over fresh in-memory state, with the request
    /// `_abc` pending.
    fn verifier() -> StrictAssertionVerifier {
        let requests = Arc::new(InMemorySaml2AuthenticationRequestRepository::default());
        requests.save("okta", "_abc", Utc::now() + Duration::minutes(5));

        StrictAssertionVerifier::new(
            Arc::new(AcceptingSignature),
            requests,
            Arc::new(InMemoryAssertionReplayCache::default()),
        )
    }

    #[test]
    fn a_response_that_holds_every_condition_is_believed() {
        let response = valid_response("_assertion", "_abc");

        StrictAssertionVerifier::new(
            Arc::new(AcceptingSignature),
            {
                let requests = Arc::new(InMemorySaml2AuthenticationRequestRepository::default());
                requests.save("okta", "_abc", Utc::now() + Duration::minutes(5));
                requests
            },
            Arc::new(InMemoryAssertionReplayCache::default()),
        )
        .verify(&registration(), &response)
        .expect("the response is believed");
    }

    #[test]
    fn a_response_whose_signature_is_refused_is_not_believed() {
        let response = valid_response("_assertion", "_abc");
        let requests = Arc::new(InMemorySaml2AuthenticationRequestRepository::default());
        requests.save("okta", "_abc", Utc::now() + Duration::minutes(5));
        let verifier = StrictAssertionVerifier::new(
            Arc::new(RefusingSignature),
            requests,
            Arc::new(InMemoryAssertionReplayCache::default()),
        );

        let error = verifier
            .verify(&registration(), &response)
            .expect_err("the signature does not cover the response");

        assert_eq!(error.kind(), Saml2ErrorKind::VerificationFailed);
    }

    #[test]
    fn a_response_with_a_failed_status_is_not_believed() {
        let not_before = (Utc::now() - Duration::minutes(1)).to_rfc3339();
        let not_on_or_after = (Utc::now() + Duration::minutes(5)).to_rfc3339();
        let response = Saml2Response::parse(&response_xml(
            "_assertion",
            "_abc",
            &not_before,
            &not_on_or_after,
            "sp-entity-id",
            "https://sp.example.org/login/saml2/sso/okta",
            "urn:oasis:names:tc:SAML:2.0:status:Requester",
        ))
        .expect("the response parses");

        let error = verifier()
            .verify(&registration(), &response)
            .expect_err("the asserting party reported a failure");

        assert_eq!(error.kind(), Saml2ErrorKind::VerificationFailed);
    }

    #[test]
    fn a_response_for_another_audience_is_not_believed() {
        let not_before = (Utc::now() - Duration::minutes(1)).to_rfc3339();
        let not_on_or_after = (Utc::now() + Duration::minutes(5)).to_rfc3339();
        let response = Saml2Response::parse(&response_xml(
            "_assertion",
            "_abc",
            &not_before,
            &not_on_or_after,
            "another-service-provider",
            "https://sp.example.org/login/saml2/sso/okta",
            "urn:oasis:names:tc:SAML:2.0:status:Success",
        ))
        .expect("the response parses");

        assert_eq!(
            verifier()
                .verify(&registration(), &response)
                .expect_err("the response is meant for someone else")
                .kind(),
            Saml2ErrorKind::VerificationFailed
        );
    }

    #[test]
    fn an_expired_response_is_not_believed() {
        let not_before = (Utc::now() - Duration::minutes(10)).to_rfc3339();
        let not_on_or_after = (Utc::now() - Duration::minutes(5)).to_rfc3339();
        let response = Saml2Response::parse(&response_xml(
            "_assertion",
            "_abc",
            &not_before,
            &not_on_or_after,
            "sp-entity-id",
            "https://sp.example.org/login/saml2/sso/okta",
            "urn:oasis:names:tc:SAML:2.0:status:Success",
        ))
        .expect("the response parses");

        assert_eq!(
            verifier()
                .verify(&registration(), &response)
                .expect_err("the assertion is over")
                .kind(),
            Saml2ErrorKind::VerificationFailed
        );
    }

    #[test]
    fn a_response_that_answers_no_request_is_not_believed() {
        let response = valid_response("_assertion", "_unknown");

        assert_eq!(
            verifier()
                .verify(&registration(), &response)
                .expect_err("the response answers no request of this application")
                .kind(),
            Saml2ErrorKind::VerificationFailed
        );
    }

    #[test]
    fn a_replayed_assertion_is_not_believed_twice() {
        let verifier = verifier();
        let response = valid_response("_assertion", "_abc");
        verifier
            .verify(&registration(), &response)
            .expect("the first submission is believed");

        let replayed = valid_response("_assertion", "_abc");

        assert_eq!(
            verifier
                .verify(&registration(), &replayed)
                .expect_err("the assertion was already used")
                .kind(),
            Saml2ErrorKind::VerificationFailed
        );
    }
}
