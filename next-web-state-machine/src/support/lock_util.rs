//! Small lock helpers shared by the state machine runtime.
//!
//! The runtime never panics on a poisoned lock: the guarded data is only ever
//! replaced or pushed to, so the state behind a poisoned lock is still usable.

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Locks a mutex, recovering the guard if the mutex was poisoned.
///
/// # Arguments
/// * `mutex` - the mutex to lock
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Acquires a read lock, recovering the guard if the lock was poisoned.
///
/// # Arguments
/// * `lock` - the lock to acquire
pub(crate) fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    match lock.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Acquires a write lock, recovering the guard if the lock was poisoned.
///
/// # Arguments
/// * `lock` - the lock to acquire
pub(crate) fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    match lock.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
