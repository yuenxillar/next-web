use std::error::Error as StdError;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use sysinfo::Disks;

use crate::actuate::health::base_health_indicator::BaseHealthIndicatorExt;
use crate::actuate::health::{Health, HealthBuilder, base_health_indicator::BaseHealthIndicator};

/// A health indicator that checks available disk space and reports a status of
/// DOWN when it drops below a configurable threshold.
pub struct DiskSpaceHealthIndicator {
    path: PathBuf,
    threshold: u64,
    inner: BaseHealthIndicator,
}

impl DiskSpaceHealthIndicator {
    /// Creates a new DiskSpaceHealthIndicator instance.
    pub fn new(path: impl Into<PathBuf>, threshold: u64) -> Self {
        Self {
            path: path.into(),
            threshold,
            inner: BaseHealthIndicator::with_message("DiskSpace health check failed"),
        }
    }

    /// Creates a new DiskSpaceHealthIndicator instance with a formatted threshold.
    pub fn with_threshold(
        path: impl Into<PathBuf>,
        threshold: impl AsRef<str>,
    ) -> Result<Self, String> {
        let threshold_bytes = Self::parse_size(threshold.as_ref())?;
        Ok(Self::new(path, threshold_bytes))
    }

    fn parse_size(size_str: &str) -> Result<u64, String> {
        let normalized = size_str.trim().replace(' ', "").to_uppercase();
        if normalized.is_empty() {
            return Err("threshold cannot be empty".to_string());
        }

        let unit_index = normalized.find(|c: char| !c.is_ascii_digit() && c != '.');
        let (number_part, unit_part) = match unit_index {
            Some(index) => (&normalized[..index], &normalized[index..]),
            None => (normalized.as_str(), ""),
        };

        if number_part.is_empty() {
            return Err(format!("invalid size: {size_str}"));
        }

        let number: f64 = number_part
            .parse()
            .map_err(|_| format!("invalid number: {number_part}"))?;

        let bytes = match unit_part {
            "" | "B" => number,
            "KB" | "K" => number * 1024.0,
            "MB" | "M" => number * 1024.0 * 1024.0,
            "GB" | "G" => number * 1024.0 * 1024.0 * 1024.0,
            "TB" | "T" => number * 1024.0 * 1024.0 * 1024.0 * 1024.0,
            _ => return Err(format!("unknown unit: {unit_part}")),
        };

        if !bytes.is_finite() || bytes < 0.0 {
            return Err(format!("invalid size: {size_str}"));
        }

        Ok(bytes as u64)
    }
}

impl Deref for DiskSpaceHealthIndicator {
    type Target = BaseHealthIndicator;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[async_trait]
impl BaseHealthIndicatorExt for DiskSpaceHealthIndicator {
    async fn do_health_check(
        &self,
        builder: HealthBuilder,
    ) -> Result<Health, Box<dyn StdError + Send + Sync>> {
        let exists = self.path.exists();
        let probe_path = resolve_probe_path(&self.path);
        let stats = disk_space_for_path(&probe_path)?;

        let mut health = if stats.free >= self.threshold {
            builder.up()
        } else {
            tracing::warn!(
                "Free disk space below threshold. path={}, available={} bytes, threshold={} bytes",
                self.path.display(),
                stats.free,
                self.threshold
            );
            builder.down()
        };

        health = health
            .with_detail("total", stats.total)
            .with_detail("free", stats.free)
            .with_detail("threshold", self.threshold)
            .with_detail("exists", exists)
            .with_detail("path", self.path.display().to_string());

        if probe_path != self.path {
            health = health.with_detail("probe_path", probe_path.display().to_string());
        }

        Ok(health.build())
    }
}

#[derive(Debug, Clone, Copy)]
struct DiskSpaceStats {
    total: u64,
    free: u64,
}

fn resolve_probe_path(path: &Path) -> PathBuf {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.exists() {
            return candidate.to_path_buf();
        }
        current = candidate.parent();
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn disk_space_for_path(path: &Path) -> Result<DiskSpaceStats, Box<dyn StdError + Send + Sync>> {
    let disks = Disks::new_with_refreshed_list();
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    let disk = disks
        .list()
        .iter()
        .filter(|disk| canonical.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .ok_or_else(|| {
            format!(
                "failed to resolve disk statistics for path {}",
                canonical.display()
            )
        })?;

    Ok(DiskSpaceStats {
        total: disk.total_space(),
        free: disk.available_space(),
    })
}

#[cfg(test)]
mod tests {
    use super::{DiskSpaceHealthIndicator, resolve_probe_path};
    use std::path::PathBuf;

    #[test]
    fn parses_human_readable_thresholds() {
        assert_eq!(
            DiskSpaceHealthIndicator::parse_size("10MB").unwrap(),
            10 * 1024 * 1024
        );
        assert_eq!(
            DiskSpaceHealthIndicator::parse_size("1.5GB").unwrap(),
            1610612736
        );
        assert_eq!(DiskSpaceHealthIndicator::parse_size("512").unwrap(), 512);
    }

    #[test]
    fn rejects_invalid_thresholds() {
        assert!(DiskSpaceHealthIndicator::parse_size("").is_err());
        assert!(DiskSpaceHealthIndicator::parse_size("ABC").is_err());
        assert!(DiskSpaceHealthIndicator::parse_size("10XB").is_err());
    }

    #[test]
    fn falls_back_to_existing_parent_path() {
        let probe = resolve_probe_path(&PathBuf::from("definitely/missing/path/for/probe"));
        assert!(probe.exists());
    }
}
