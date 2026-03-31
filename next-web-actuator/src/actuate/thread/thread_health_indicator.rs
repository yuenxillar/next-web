use std::error::Error as StdError;
use std::ops::Deref;

use async_trait::async_trait;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, get_current_pid};

use crate::actuate::health::base_health_indicator::BaseHealthIndicatorExt;
use crate::actuate::health::{Health, HealthBuilder, base_health_indicator::BaseHealthIndicator};

/// A health indicator that inspects the current process thread usage.
///
/// On platforms where the underlying process API exposes thread/task lists, the
/// indicator compares the current thread count against `max_threads`.
/// On other platforms, it reports the process-level metrics that are available
/// and marks the thread count as unsupported instead of fabricating a value.
pub struct ThreadHealthIndicator {
    max_threads: usize,
    inner: BaseHealthIndicator,
}

impl ThreadHealthIndicator {
    /// Creates a new thread health indicator with an explicit thread threshold.
    pub fn new(max_threads: usize) -> Self {
        Self {
            max_threads: max_threads.max(1),
            inner: BaseHealthIndicator::with_message("Thread health check failed"),
        }
    }

    /// Creates a thread health indicator using a pragmatic default threshold.
    ///
    /// The default is `available_parallelism * 32`, with a floor of `64`.
    pub fn with_default_threshold() -> Self {
        let available = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1);
        let threshold = available.saturating_mul(32).max(64);
        Self::new(threshold)
    }

    fn evaluate(thread_count: Option<usize>, max_threads: usize) -> ThreadHealthEvaluation {
        match thread_count {
            Some(thread_count) => ThreadHealthEvaluation {
                status_up: thread_count <= max_threads,
                overload: thread_count.saturating_sub(max_threads),
            },
            None => ThreadHealthEvaluation {
                status_up: true,
                overload: 0,
            },
        }
    }
}

impl Default for ThreadHealthIndicator {
    fn default() -> Self {
        Self::with_default_threshold()
    }
}

impl Deref for ThreadHealthIndicator {
    type Target = BaseHealthIndicator;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[async_trait]
impl BaseHealthIndicatorExt for ThreadHealthIndicator {
    async fn do_health_check(
        &self,
        builder: HealthBuilder,
    ) -> Result<Health, Box<dyn StdError + Send + Sync>> {
        let pid = get_current_pid()?;
        let mut system = System::new();
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_tasks(),
        );

        let process = system.process(pid).ok_or_else(|| {
            format!(
                "failed to resolve current process information for pid {}",
                pid.as_u32()
            )
        })?;

        let thread_count = process.tasks().map(|tasks| tasks.len());
        let available_parallelism = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1);
        let evaluation = Self::evaluate(thread_count, self.max_threads);

        if !evaluation.status_up {
            tracing::warn!(
                "Process thread count exceeded threshold. pid={}, threads={}, threshold={}",
                pid.as_u32(),
                thread_count.unwrap_or_default(),
                self.max_threads
            );
        }

        let mut health = if evaluation.status_up {
            builder.up()
        } else {
            builder.down()
        };

        health = health
            .with_detail("pid", pid.as_u32())
            .with_detail("process_name", process.name().to_string_lossy().to_string())
            .with_detail("cpu_usage", process.cpu_usage())
            .with_detail("memory", process.memory())
            .with_detail("virtual_memory", process.virtual_memory())
            .with_detail("available_parallelism", available_parallelism)
            .with_detail("max_threads", self.max_threads)
            .with_detail("thread_count_supported", thread_count.is_some())
            .with_detail("thread_count", thread_count)
            .with_detail("status_basis", status_basis(thread_count))
            .with_detail("platform", std::env::consts::OS);

        if evaluation.overload > 0 {
            health = health.with_detail("overload_threads", evaluation.overload);
        }

        Ok(health.build())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThreadHealthEvaluation {
    status_up: bool,
    overload: usize,
}

fn status_basis(thread_count: Option<usize>) -> &'static str {
    if thread_count.is_some() {
        "current-process-thread-count"
    } else {
        "current-process-metrics-only"
    }
}

#[cfg(test)]
mod tests {
    use super::{ThreadHealthIndicator, status_basis};

    #[test]
    fn evaluate_stays_up_within_threshold() {
        let evaluation = ThreadHealthIndicator::evaluate(Some(8), 16);
        assert!(evaluation.status_up);
        assert_eq!(evaluation.overload, 0);
    }

    #[test]
    fn evaluate_goes_down_over_threshold() {
        let evaluation = ThreadHealthIndicator::evaluate(Some(18), 16);
        assert!(!evaluation.status_up);
        assert_eq!(evaluation.overload, 2);
    }

    #[test]
    fn evaluate_stays_up_when_thread_count_is_unavailable() {
        let evaluation = ThreadHealthIndicator::evaluate(None, 16);
        assert!(evaluation.status_up);
        assert_eq!(evaluation.overload, 0);
        assert_eq!(status_basis(None), "current-process-metrics-only");
    }

    #[test]
    fn default_threshold_is_positive() {
        let indicator = ThreadHealthIndicator::default();
        assert!(indicator.max_threads > 0);
    }
}
