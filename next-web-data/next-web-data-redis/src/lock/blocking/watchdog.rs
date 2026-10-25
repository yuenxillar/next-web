//! The watchdog thread that keeps a held lease alive.
//!
//! The blocking flavour cannot spawn a task, so one OS thread renews one lease.
//! It stops when the guard releases the lock or is dropped: dropping the
//! [`std::sync::mpsc::Sender`] disconnects the channel, which wakes
//! `recv_timeout` with `Disconnected`.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{RecvTimeoutError, Sender, channel},
    },
    time::Duration,
};

use tracing::warn;

use crate::lock::{
    backend::BlockingLockBackend, error::RedisLockError, lease::Lease, ownership::OwnerToken,
    protocol::commands::renew,
};

/// A running renewal thread.
pub(crate) struct RenewalThread {
    stop: Sender<()>,
}

impl RenewalThread {
    /// Start renewing `name` on `backend` every lease/3.
    pub(crate) fn spawn(
        backend: Arc<dyn BlockingLockBackend>,
        name: Arc<str>,
        owner: Arc<OwnerToken>,
        lease: Lease,
        lost: Arc<AtomicBool>,
        command_timeout: Duration,
    ) -> Self {
        let (stop, stopped) = channel();
        let interval = lease.renewal_interval();

        std::thread::spawn(move || {
            loop {
                match stopped.recv_timeout(interval) {
                    // An explicit stop, or the guard dropped the sender.
                    Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {
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
                            match backend.eval(&request, command_timeout) {
                                Ok(reply) if reply.int() == Some(1) => {
                                    renewed = true;
                                    break;
                                }
                                // 0 means this owner no longer holds a level.
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

        Self { stop }
    }
}

impl Drop for RenewalThread {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}
