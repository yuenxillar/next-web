//! Unique ID generators for `next-web`.
//!
//! This crate provides several ID generation strategies:
//! - [`SnowflakeGenerator`] for distributed sortable numeric IDs
//! - [`MonotonicIdGenerator`] for lightweight strictly increasing local IDs
//! - [`UuidGenerator`] for UUID v4 strings
//! - [`RandomStringIdGenerator`] for short random string IDs

mod error;
mod generator;
mod monotonic;
mod random_string;
mod snowflake;
mod uuid;

pub use error::IdGeneratorError;
pub use generator::IdGenerator;
pub use monotonic::MonotonicIdGenerator;
pub use random_string::RandomStringIdGenerator;
pub use snowflake::{ClockRollbackStrategy, SnowflakeConfig, SnowflakeGenerator, SnowflakeId};
pub use uuid::UuidGenerator;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::thread;

    use super::*;

    #[test]
    fn monotonic_ids_are_strictly_increasing() {
        let generator = MonotonicIdGenerator::new();
        let first = generator.next_id().unwrap();
        let second = generator.next_id().unwrap();
        let third = generator.next_id().unwrap();

        assert!(first < second);
        assert!(second < third);
    }

    #[test]
    fn snowflake_ids_are_unique_under_contention() {
        let generator = Arc::new(SnowflakeGenerator::new(7).unwrap());
        let mut handles = Vec::new();

        for _ in 0..8 {
            let generator = Arc::clone(&generator);
            handles.push(thread::spawn(move || {
                let mut ids = Vec::with_capacity(1_000);
                for _ in 0..1_000 {
                    ids.push(generator.next_id().unwrap());
                }
                ids
            }));
        }

        let mut all_ids = HashSet::with_capacity(8_000);
        for handle in handles {
            for id in handle.join().unwrap() {
                assert!(all_ids.insert(id), "duplicate snowflake id: {id}");
            }
        }
    }

    #[test]
    fn snowflake_parts_can_be_parsed() {
        let generator = SnowflakeGenerator::with_config(
            SnowflakeConfig::new(12).with_epoch_millis(1_577_836_800_000),
        )
        .unwrap();
        let raw = generator.next_id().unwrap();
        let parsed = generator.parse(raw);

        assert_eq!(parsed.raw(), raw);
        assert_eq!(parsed.node_id(), 12);
        assert!(parsed.timestamp_millis(generator.config().epoch_millis()) >= 1_577_836_800_000);
    }

    #[test]
    fn invalid_node_id_is_rejected() {
        let err = SnowflakeGenerator::new(2_048).unwrap_err();
        assert!(matches!(err, IdGeneratorError::InvalidNodeId { .. }));
    }

    #[test]
    fn random_string_generator_respects_configuration() {
        let generator = RandomStringIdGenerator::new("ABC123", 32).unwrap();
        let value = generator.next_id().unwrap();

        assert_eq!(value.len(), 32);
        assert!(value.chars().all(|ch| "ABC123".contains(ch)));
    }

    #[test]
    fn uuid_generator_formats_are_supported() {
        let generator = UuidGenerator;
        let hyphenated = generator.next_hyphenated();
        let compact = generator.next_compact();

        assert_eq!(hyphenated.len(), 36);
        assert_eq!(compact.len(), 32);
        assert!(hyphenated.contains('-'));
        assert!(!compact.contains('-'));
    }
}
