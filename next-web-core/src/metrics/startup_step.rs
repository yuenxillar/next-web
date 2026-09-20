//! Step recording metrics about a particular phase or action happening during
//! the [`ApplicationStartup`].
//!
//! # Lifecycle
//!
//! The lifecycle of a [`StartupStep`] goes as follows:
//!
//! 1. The step is created and starts by calling [`ApplicationStartup::start`]
//!    and is assigned a unique [`StartupStep::id`].
//! 2. Information can then be attached via [`StartupStep::tag`] during
//!    processing.
//! 3. The step must then be marked as ended via [`StartupStep::end`].
//!
//! Implementations can track the "execution time" or other metrics for steps.

use std::collections::HashMap;

/// Step recording metrics about a particular phase or action happening during
/// the [`ApplicationStartup`].
///
/// # Lifecycle
///
/// See the module-level documentation for the full lifecycle.
///
/// # Closing
///
/// In Java, this interface extends `AutoCloseable` and its `close` method
/// delegates to [`Self::end`]. In Rust, there is no direct equivalent, but
/// implementors may call [`Self::end`] from their `Drop` implementation if
/// desired, or callers can use a scope guard.
pub trait StartupStep: Send + Sync {
    /// Returns the name of the startup step.
    ///
    /// A step name describes the current action or phase. This technical name
    /// should be "." namespaced and can be reused to describe other instances
    /// of similar steps during application startup.
    ///
    /// # Returns
    ///
    /// The step name.
    fn name(&self) -> &str;

    /// Returns the unique id for this step within the application startup.
    ///
    /// # Returns
    ///
    /// The step id.
    fn id(&self) -> u64;

    /// Returns, if available, the id of the parent step.
    ///
    /// The parent step is the step that was started most recently when the
    /// current step was created.
    ///
    /// # Returns
    ///
    /// The parent step id, or `None` if there is no parent.
    fn parent_id(&self) -> Option<u64>;

    /// Adds a [`Tag`] to the step.
    ///
    /// # Arguments
    ///
    /// * `key` - The tag key.
    /// * `value` - The tag value.
    ///
    /// # Returns
    ///
    /// `self`, to allow chained calls.
    fn tag(&mut self, key: &str, value: String) -> &mut dyn StartupStep;

    /// Adds a [`Tag`] to the step using a lazy value supplier.
    ///
    /// The supplier is only invoked if the implementation actually records
    /// tags, avoiding unnecessary computation in no-op implementations.
    ///
    /// # Arguments
    ///
    /// * `key` - The tag key.
    /// * `value` - A closure producing the tag value.
    ///
    /// # Returns
    ///
    /// `self`, to allow chained calls.
    fn tag_lazy(&mut self, key: &str, value: &dyn FnOnce() -> String) -> &mut dyn StartupStep;

    /// Returns the [`Tags`] collection for this step.
    ///
    /// # Returns
    ///
    /// The tags attached to this step.
    fn tags(&self) -> Tags<'_>;

    /// Records the state of the step and possibly other metrics like execution
    /// time.
    ///
    /// Once ended, changes on the step state are not allowed.
    fn end(&mut self);

    /// Closes the step, delegating to [`Self::end`].
    ///
    /// This mirrors the Java `AutoCloseable.close` default method, allowing
    /// callers to treat steps as scope-bound resources.
    fn close(&mut self) {
        self.end();
    }
}

/// An immutable collection of [`Tag`] values.
///
/// This is a lightweight view over the tags attached to a [`StartupStep`].
/// Implementations typically borrow from the step rather than owning the data.
#[derive(Debug, Clone, Copy)]
pub struct Tags<'a> {
    /// The borrowed tag entries.
    entries: &'a [Tag],
}

impl<'a> Tags<'a> {
    /// Creates a new [`Tags`] view over the given entries.
    ///
    /// # Arguments
    ///
    /// * `entries` - The tag entries.
    ///
    /// # Returns
    ///
    /// A new tags view.
    pub fn new(entries: &'a [Tag]) -> Self {
        Self { entries }
    }

    /// Returns an empty tags view.
    ///
    /// This is useful for no-op implementations that never store tags.
    ///
    /// # Returns
    ///
    /// An empty [`Tags`] view.
    pub fn empty() -> Self {
        Self { entries: &[] }
    }

    /// Returns the number of tags.
    ///
    /// # Returns
    ///
    /// The tag count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether there are no tags.
    ///
    /// # Returns
    ///
    /// `true` if there are no tags.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns an iterator over the tags.
    ///
    /// # Returns
    ///
    /// An iterator yielding [`&Tag`](Tag) references.
    pub fn iter(&self) -> std::slice::Iter<'_, Tag> {
        self.entries.iter()
    }

    /// Returns the underlying slice of tags.
    ///
    /// # Returns
    ///
    /// The tag slice.
    pub fn as_slice(&self) -> &[Tag] {
        self.entries
    }
}

impl<'a> IntoIterator for Tags<'a> {
    type Item = &'a Tag;
    type IntoIter = std::slice::Iter<'a, Tag>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

/// Simple key/value association for storing step metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag {
    /// The tag key.
    key: String,
    /// The tag value.
    value: String,
}

impl Tag {
    /// Creates a new [`Tag`].
    ///
    /// # Arguments
    ///
    /// * `key` - The tag key.
    /// * `value` - The tag value.
    ///
    /// # Returns
    ///
    /// A new tag.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    /// Returns the [`Tag`] key.
    ///
    /// # Returns
    ///
    /// The tag key.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the [`Tag`] value.
    ///
    /// # Returns
    ///
    /// The tag value.
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// A simple owned collection of [`Tag`] values.
///
/// Useful for implementations that need to store tags and later expose them
/// through [`StartupStep::tags`].
#[derive(Debug, Default, Clone)]
pub struct OwnedTags {
    /// The owned tag entries.
    entries: Vec<Tag>,
}

impl OwnedTags {
    /// Creates an empty [`OwnedTags`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces a tag.
    ///
    /// # Arguments
    ///
    /// * `key` - The tag key.
    /// * `value` - The tag value.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        if let Some(existing) = self.entries.iter_mut().find(|t| t.key == key) {
            existing.value = value;
        } else {
            self.entries.push(Tag { key, value });
        }
    }

    /// Returns a borrowed view over the stored tags.
    ///
    /// # Returns
    ///
    /// A [`Tags`] view.
    pub fn as_tags(&self) -> Tags<'_> {
        Tags::new(&self.entries)
    }

    /// Returns the underlying entries.
    ///
    /// # Returns
    ///
    /// The tag slice.
    pub fn as_slice(&self) -> &[Tag] {
        &self.entries
    }

    /// Returns whether there are no tags.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the number of tags.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl From<HashMap<String, String>> for OwnedTags {
    fn from(map: HashMap<String, String>) -> Self {
        Self {
            entries: map
                .into_iter()
                .map(|(key, value)| Tag { key, value })
                .collect(),
        }
    }
}
