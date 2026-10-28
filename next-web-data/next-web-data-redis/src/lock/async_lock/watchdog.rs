//! The watchdog that keeps a held lease alive.
//!
//! One task renews one lease. It stops when the guard releases the lock or is
//! dropped, and it marks the lease lost when it can no longer prove ownership,
//! which is the only signal the lock layer has that the lease may have expired
//! under it.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tokio::sync::watch;
use tracing::warn;

use crate::lock::{
    backend::LockBackend, error::RedisLockError, lease::Lease, ownership::OwnerToken,
    protocol::commands::renew,
};

/// A running renewal task.
pub(crate) struct RenewalTask {
    cancel: watch::Sender<bool>,
}

impl RenewalTask {
    /// Start renewing `name` on `backend` every lease/3.
    ///
    /// `lost` is the flag of the [`super::super::lease::NodeLease`] this task
    /// keeps alive; it is set when two renewals in a row fail.
    pub(crate) fn spawn(
        backend: Arc<dyn LockBackend>,
        name: Arc<str>,
        owner: Arc<OwnerToken>,
        lease: Lease,
        lost: Arc<AtomicBool>,
        command_timeout: Duration,
    ) -> Self {
        let (cancel, mut cancelled) = watch::channel(false);

        tokio::spawn(async move {
            let interval = lease.renewal_interval();

            loop {
                tokio::select! {
                    changed = cancelled.changed() => {
                        if changed.is_err() || *cancelled.borrow() {
                            break;
                        }
                    }
                    () = tokio::time::sleep(interval) => {
                        let request = match renew(&name, owner.as_str(), lease.duration) {
                            Ok(request) => request,
                            Err(error) => {
                                warn!(lock = %name, %error, "the lock watchdog cannot build a renewal");
                                lost.store(true, Ordering::Release);
                                break;
                            }
                        };

                        let mut renewed = false;
                        for _ in 0..2 {
                            match backend.eval(&request, command_timeout).await {
                                Ok(reply) if reply.int() == Some(1) => {
                                    renewed = true;
                                    break;
                                }
                                // 0 means this owner no longer holds a level: there
                                // is nothing left to renew.
                                Ok(_) => break,
                                Err(RedisLockError::CommandTimeout { .. }) => continue,
                                Err(error) => {
                                    warn!(lock = %name, %error, "the lock watchdog could not renew");
                                    continue;
                                }
                            }
                        }

                        if !renewed {
                            warn!(
                                lock = %name,
                                owner = owner.as_str(),
                                "the lock watchdog lost ownership of its lease"
                            );
                            lost.store(true, Ordering::Release);
                            break;
                        }
                    }
                }
            }
        });

        Self { cancel }
    }

    /// Stop renewing.
    ///
    /// Sending on a watch channel is synchronous, which is what lets `Drop`
    /// call this: a guard dropped without an explicit unlock must still stop the
    /// watchdog even though it cannot perform IO.
    pub(crate) fn stop(&self) {
        let _ = self.cancel.send(true);
    }
}

impl Drop for RenewalTask {
    fn drop(&mut self) {
        self.stop();
    }
}
