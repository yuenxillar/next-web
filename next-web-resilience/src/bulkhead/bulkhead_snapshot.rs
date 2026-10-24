//! A point in time view of a bulkhead's counters.

/// The counters a bulkhead keeps, as of one instant.
///
/// The available and maximum slot counts describe the bulkhead's capacity right
/// now; the rejection total accumulates since the bulkhead was created and is
/// never reset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BulkheadSnapshot {
    /// Number of slots free at the moment of the snapshot.
    pub available_concurrent_calls: u32,
    /// Number of slots the bulkhead was configured with.
    pub max_allowed_concurrent_calls: u32,
    /// Total number of calls rejected because no slot was free in time.
    pub number_of_rejected_calls: u64,
}

impl BulkheadSnapshot {
    /// Returns whether every slot is taken.
    pub fn is_full(&self) -> bool {
        self.available_concurrent_calls == 0
    }
}

#[cfg(test)]
mod tests {
    use super::BulkheadSnapshot;

    fn snapshot(available_concurrent_calls: u32) -> BulkheadSnapshot {
        BulkheadSnapshot {
            available_concurrent_calls,
            max_allowed_concurrent_calls: 25,
            number_of_rejected_calls: 0,
        }
    }

    #[test]
    fn a_bulkhead_without_a_free_slot_is_full() {
        assert!(snapshot(0).is_full());
        assert!(!snapshot(1).is_full());
    }
}
