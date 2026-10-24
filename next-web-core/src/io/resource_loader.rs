use std::{borrow::Cow, io};

use crate::io::Resource;

/// Strategy interface for loading resources (for example, class path or file system
/// resources).
///
/// A context environment is required to provide this functionality plus extended
/// resource pattern resolver support.
///
/// [`DefaultResourceLoader`] is a standalone implementation that is usable outside
/// a context and is also used by `ResourceEditor`.
///
/// Bean properties of type `Resource` and `Resource[]` can be populated from Strings
/// when running in a context, using the particular context's resource loading strategy.
pub trait ResourceLoader
where
    Self: Send + Sync,
{
    /// Load resources into the cache.
    ///
    /// Performs the initial (or full) scan of the underlying source and populates
    /// the internal cache so that subsequent lookups can be served without touching
    /// the physical medium again.
    ///
    /// This is intended to be called once during initialization. Implementations
    /// should make this operation idempotent, or at least safe to call again.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if the underlying source cannot be accessed or
    /// partially read.
    fn load(&mut self) -> io::Result<()>;

    /// Refresh the cache from the underlying source.
    ///
    /// Unlike [`load`](Self::load), this is meant to be called when the source
    /// may have changed at runtime (for example, files added, removed, or modified).
    /// Implementations typically re-scan the source and rebuild or incrementally
    /// update the internal cache.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if the underlying source cannot be accessed or
    /// partially read.
    fn refresh(&mut self) -> io::Result<()>;

    /// Clear all cached resources.
    ///
    /// Drops every entry held in the internal cache. The loader remains usable,
    /// but lookups will fail until [`load`](Self::load) or
    /// [`refresh`](Self::refresh) is called again.
    fn clear(&mut self);

    /// Return a [`Resource`] handle for the specified resource location.
    ///
    /// `location` is a logical path relative to the loader's root
    /// (for example, `"messages/en.yaml"`). No pattern matching is performed;
    /// the location is resolved exactly.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if the resource does not exist or cannot be
    /// accessed.
    fn get_resource(&self, location: &str) -> io::Result<&dyn Resource>;

    /// Return all [`Resource`] handles matching the given pattern.
    ///
    /// Supports glob-style patterns such as `"messages/*.yaml"` or
    /// `"messages/**/*.yaml"`. Pattern matching is performed against the
    /// cached paths, so [`load`](Self::load) should have been called first.
    ///
    /// The returned slice is ordered by path for deterministic results.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if the pattern is invalid or the underlying
    /// index cannot be queried.
    fn get_resources(&self, pattern: &str) -> io::Result<Vec<&dyn Resource>>;

    /// Return a [`Resource`] handle for the specified directory location.
    ///
    /// Returns the direct entries (names) contained in `location`. The entries
    /// are borrowed strings tied to the loader's cache, so they are cheap to
    /// produce and do not allocate when the loader owns the names.
    ///
    /// # Errors
    ///
    /// This method does not return a `Result`; callers should use
    /// [`exists`](Self::exists) beforehand if they need to distinguish a missing
    /// directory from an empty one.
    fn get_directory(&self, location: &str) -> Vec<Cow<'_, str>>;

    /// Return all paths for the specified directory location.
    ///
    /// Unlike [`get_directory`](Self::get_directory), which lists direct
    /// entries, this returns every known path (including nested ones) held by
    /// the loader, optionally relative to `location`.
    ///
    /// The returned strings borrow from the loader's cache.
    fn paths(&self) -> Vec<Cow<'_, str>>;

    /// Determine whether the specified resource exists in physical form.
    ///
    /// This checks the underlying cache/index. It does not guarantee that the
    /// resource is readable, only that it was discovered by the loader.
    fn exists(&self, location: &str) -> bool;
}
