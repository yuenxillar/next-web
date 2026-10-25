//! The bookkeeping both guard flavours share.
//!
//! A guard owns the level an acquisition added, the owner token it added it
//! under, and one [`NodeLease`] per node that granted it. The two public guard
//! types (async and blocking) wrap this and differ only in how they release.

use std::sync::Arc;

use crate::lock::lease::NodeLease;
use crate::lock::ownership::OwnerToken;

/// What a guard knows about the acquisition it represents.
pub(crate) struct GuardCore {
    /// The lock name.
    pub(crate) name: Arc<str>,
    /// The owner token the lock was taken under.
    pub(crate) owner: OwnerToken,
    /// How many nodes must still hold the lease for the guard to be valid.
    pub(crate) quorum: usize,
    /// One lease per node that granted it.
    pub(crate) leases: Vec<NodeLease>,
}

impl GuardCore {
    /// Record a completed acquisition.
    pub(crate) fn new(
        name: Arc<str>,
        owner: OwnerToken,
        quorum: usize,
        leases: Vec<NodeLease>,
    ) -> Self {
        Self {
            name,
            owner,
            quorum,
            leases,
        }
    }

    /// Whether too many nodes lost the lease for this guard to still be valid.
    pub(crate) fn is_lost(&self) -> bool {
        self.leases.len().saturating_sub(self.lost()) < self.quorum
    }

    /// How many nodes lost their lease.
    pub(crate) fn lost(&self) -> usize {
        self.leases.iter().filter(|lease| lease.is_lost()).count()
    }

    /// The lease granted on the first node, which is the one every node shares.
    pub(crate) fn lease(&self) -> crate::lock::lease::Lease {
        self.leases
            .first()
            .map(|lease| lease.lease)
            .unwrap_or_else(|| crate::lock::lease::Lease::fixed(std::time::Duration::ZERO))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;
    use crate::lock::lease::Lease;
    use crate::lock::ownership::LockOwner;

    fn core(leases: Vec<NodeLease>) -> GuardCore {
        GuardCore::new(
            Arc::from("orders"),
            LockOwner::default().next_token(),
            2,
            leases,
        )
    }

    #[test]
    fn a_guard_tolerates_a_minority_of_lost_leases() {
        let leases = vec![
            NodeLease::new(0, Lease::watchdog(Duration::from_secs(30))),
            NodeLease::new(1, Lease::watchdog(Duration::from_secs(30))),
            NodeLease::new(2, Lease::watchdog(Duration::from_secs(30))),
        ];
        let core = core(leases);

        // 3 nodes, quorum 2: one loss is tolerated.
        core.leases[0].mark_lost();
        assert!(!core.is_lost());
        assert_eq!(core.lost(), 1);

        // Two losses drop below the quorum.
        core.leases[1].mark_lost();
        assert!(core.is_lost());
        assert_eq!(core.lost(), 2);
    }
}
