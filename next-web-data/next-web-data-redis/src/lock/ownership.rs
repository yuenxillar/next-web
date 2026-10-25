//! Owner tokens: the identity a lock handle acquires and releases under.
//!
//! Redisson identifies a lock holder by thread. A lock can be held across
//! `.await` points, where no thread identity survives, so this crate uses an
//! explicit token instead: [`LockOwner::next_token`] is called once when a lock
//! handle is created, and a clone of that handle shares the token. Reentrancy
//! therefore follows the handle, not the thread.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use uuid::Uuid;

/// The token a lock handle acquires and releases under.
///
/// The wire form is `"<client-id>:<sequence>"`, which is also the Redis hash
/// field Redisson writes, so the two implementations stay interchangeable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnerToken(Arc<str>);

impl OwnerToken {
    /// The token in its wire form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Mints owner tokens for every handle one service creates.
///
/// The client id is stable for the life of the service and the sequence is
/// shared by its handles, so two services never mint the same token.
#[derive(Debug, Clone)]
pub struct LockOwner {
    client_id: Arc<str>,
    sequence: Arc<AtomicU64>,
}

impl Default for LockOwner {
    fn default() -> Self {
        Self::new()
    }
}

impl LockOwner {
    /// Create an owner with a fresh random client id.
    pub fn new() -> Self {
        Self {
            client_id: Uuid::new_v4().to_string().into(),
            sequence: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Mint the next token. Each call returns a token no other call returns.
    pub fn next_token(&self) -> OwnerToken {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        OwnerToken(format!("{}:{sequence}", self.client_id).into())
    }

    /// The client id every token from this owner starts with.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_token_is_distinct_and_stable() {
        let owner = LockOwner::default();
        let first = owner.next_token();
        let second = owner.next_token();

        assert_ne!(first, second);
        assert_eq!(first.as_str(), first.clone().as_str());
        assert!(first.as_str().contains(':'));
    }

    #[test]
    fn two_owners_never_share_a_prefix() {
        let first = LockOwner::new();
        let second = LockOwner::new();

        assert_ne!(first.client_id(), second.client_id());
        assert!(first.next_token().as_str().starts_with(first.client_id()));
    }
}
