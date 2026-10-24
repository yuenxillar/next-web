use std::ops::{Deref, DerefMut};

use crate::ApplicationState;
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use next_web_context::ApplicationContextExt;
use next_web_singletons::util::SingletonUtil;

/// Axum extractor that looks up and injects a singleton from `ApplicationState` by type.
///
/// Usage: declare `FindSingleton<MyService>` as a handler argument, and Axum will
/// automatically invoke `from_request_parts` before entering the handler.
///
/// The inner value is exposed as `pub T` and paired with `Deref`/`DerefMut` so
/// callers can use it as if it were a `MyService` directly (e.g. `svc.do_something()`).
#[derive(Clone)]
pub struct FindSingleton<T>(pub T);

/// Implements Axum's `FromRequestParts`, enabling `FindSingleton<T>` to be used as an extractor.
///
/// Trait bounds:
/// - `S: Send + Sync` — required by Axum for the state type.
/// - `T: Send + Sync + 'static` — the singleton must be shareable across threads
///   and free of borrowed lifetimes.
/// - `T: Clone` — the instance retrieved from state is cloned for the current request.
impl<S, T> FromRequestParts<S> for FindSingleton<T>
where
    S: Send + Sync,
    T: Send + Sync + 'static,
    T: Clone,
{
    /// On failure, return a bare 500 without exposing internal details to the client.
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(req: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Pull the application state out of request extensions
        // (injected earlier by an upstream middleware).
        let state = req
            .extensions
            .get::<ApplicationState>()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"))?;

        // Derive the default singleton name from type `T`
        // (typically the fully-qualified type name).
        let name = SingletonUtil::default_singleton_name::<T>();

        // Try to find or resolve the singleton; on failure, return a uniform 500.
        let instance = find(state, name)
            .await
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"))?;

        Ok(Self(instance))
    }
}

/// Allows transparent access to the inner singleton via `*svc` or method-call syntax.
impl<T> Deref for FindSingleton<T>
where
    T: Clone,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Allows mutable access to the inner singleton.
///
/// Note: singletons are usually shared instances. If mutation of internal state
/// is required, the type should wrap `Mutex`/`RwLock` or similar synchronization
/// primitives internally.
impl<T> DerefMut for FindSingleton<T>
where
    T: Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Looks up a singleton by name in `ApplicationState`; if absent, attempts to
/// resolve and register it.
///
/// Lookup strategy (in order):
/// 1. Exact lookup using the provided `name`.
/// 2. Fallback lookup using the empty string `""` as the default name
///    (for compatibility with unnamed registrations).
/// 3. If still not found, acquire the write lock and call
///    `resolve_option_with_name` to trigger creation/resolution.
///
/// Locking notes:
/// - The read lock is held only during the lookup phase. Once a hit occurs,
///   `drop(reader)` releases it immediately to avoid a deadlock with a
///   subsequent write-lock acquisition.
/// - The write lock is held only briefly, and only when a new instance must
///   be created.
pub async fn find<T>(state: &ApplicationState, name: String) -> Option<T>
where
    T: Send + Sync,
    T: Clone + 'static,
{
    // Acquire a read lock: multiple requests may concurrently look up
    // existing singletons.
    let reader = state.read().await;

    // First, try the provided name.
    if let Some(instance) = reader
        .get_singleton_option_with_name::<T>(name.to_owned())
        .map(Clone::clone)
    {
        return Some(instance);
    }

    // Fallback: look up by the empty name (default singleton name).
    if let Some(instance) = reader
        .get_singleton_option_with_name::<T>("")
        .map(Clone::clone)
    {
        return Some(instance);
    }

    // Explicitly release the read lock; otherwise the subsequent `write()`
    // would block forever on the reader-writer mutex (self-deadlock).
    drop(reader);

    // Acquire the write lock only when a new singleton must be created/resolved,
    // keeping the contention window small.
    let mut writer = state.write().await;

    writer.resolve_option_with_name::<T>(name)
}
