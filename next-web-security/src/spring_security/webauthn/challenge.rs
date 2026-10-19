//! The challenges a ceremony answers.
//!
//! A challenge is the only thing that makes a ceremony a *fresh* one: the browser
//! signs it, so a challenge that was already answered must never be accepted
//! again. It is therefore stored before it is handed out and consumed when it is
//! answered — one use, one window.

use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{DateTime, Utc};

/// The challenges a relying party issued and has not accepted yet.
pub trait ChallengeRepository: Send + Sync {
    /// Remembers a challenge until it expires.
    fn save(&self, challenge: &[u8], expires_at: DateTime<Utc>);

    /// Consumes a challenge, and returns whether it was known and unexpired.
    ///
    /// A challenge is consumed whether or not the ceremony that answered it turns
    /// out to be acceptable: an authenticator that answered a challenge used it
    /// up, and a retry has to start a new ceremony.
    fn consume(&self, challenge: &[u8]) -> bool;
}

/// A [`ChallengeRepository`] that holds the challenges in memory.
#[derive(Debug, Default)]
pub struct InMemoryChallengeRepository {
    challenges: RwLock<HashMap<Vec<u8>, DateTime<Utc>>>,
}

impl ChallengeRepository for InMemoryChallengeRepository {
    fn save(&self, challenge: &[u8], expires_at: DateTime<Utc>) {
        let mut challenges = self
            .challenges
            .write()
            .expect("the challenge store lock is poisoned");

        challenges.retain(|_, expiry| *expiry > Utc::now());
        challenges.insert(challenge.to_vec(), expires_at);
    }

    fn consume(&self, challenge: &[u8]) -> bool {
        let mut challenges = self
            .challenges
            .write()
            .expect("the challenge store lock is poisoned");
        let Some(expires_at) = challenges.remove(challenge) else {
            return false;
        };

        expires_at > Utc::now()
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn a_challenge_that_was_issued_is_accepted_once() {
        let repository = InMemoryChallengeRepository::default();
        repository.save(b"the-challenge", Utc::now() + Duration::minutes(5));

        assert!(repository.consume(b"the-challenge"), "the first answer is accepted");
        assert!(
            !repository.consume(b"the-challenge"),
            "the second answer of the same challenge is a replay"
        );
    }

    #[test]
    fn a_challenge_that_was_never_issued_is_refused() {
        let repository = InMemoryChallengeRepository::default();

        assert!(!repository.consume(b"another-challenge"));
    }

    #[test]
    fn an_expired_challenge_is_refused() {
        let repository = InMemoryChallengeRepository::default();
        repository.save(b"the-challenge", Utc::now() - Duration::minutes(1));

        assert!(!repository.consume(b"the-challenge"));
    }
}
