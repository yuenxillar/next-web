use std::{any::TypeId, io};

use crate::core::poi::poifs::filesystem::{
    document_entry::DocumentEntry, poifs_writer_listener::POIFSWriterListener,
};

/// This interface defines methods specific to Directory objects
/// managed by a Filesystem instance.
pub trait DirectoryEntry: Entry + std::iter::Iterator<Item = Box<dyn Entry>> {
    /// Get an iterator of the Entry instances contained directly in
    /// this instance (in other words, children only; no grandchildren etc.)
    ///
    /// # Returns
    /// Iterator; never null, but may be empty (i.e., this DirectoryEntry is empty).
    /// All objects retrieved are guaranteed to be implementations of Entry.
    fn entries(&self) -> Box<dyn Iterator<Item = Box<dyn Entry>> + '_>;

    /// Get the names of all the Entries contained directly in this
    /// instance (in other words, names of children only; no grandchildren etc).
    ///
    /// # Returns
    /// The names of all the entries that may be retrieved with `get_entry()`,
    /// which may be empty (if this DirectoryEntry is empty)
    fn entry_names(&self) -> std::collections::HashSet<String>;

    /// Check if this DirectoryEntry is empty?
    ///
    /// # Returns
    /// `true` if this instance contains no Entry instances
    fn is_empty(&self) -> bool;

    /// Find out how many Entry instances are contained directly within
    /// this DirectoryEntry
    ///
    /// # Returns
    /// Number of immediately (no grandchildren etc.) contained Entry instances
    fn entry_count(&self) -> usize;

    /// Checks if entry with specified name present, case sensitive
    fn has_entry(&self, name: &str) -> bool;

    /// Checks if entry with specified name present, case insensitive
    fn has_entry_case_insensitive(&self, name: &str) -> bool;

    /// Get a specified Entry by name, case sensitive
    ///
    /// # Arguments
    /// * `name` - the name of the Entry to obtain.
    ///
    /// # Returns
    /// The specified Entry, if it is directly contained in this DirectoryEntry
    ///
    /// # Errors
    /// Returns `io::Error` with `io::ErrorKind::NotFound` if no Entry with the
    /// specified name exists in this DirectoryEntry
    fn get_entry(&self, name: &str) -> io::Result<Box<dyn Entry>>;

    /// Get a specified Entry by name, case insensitive
    ///
    /// # Arguments
    /// * `name` - the name of the Entry to obtain.
    ///
    /// # Returns
    /// The specified Entry, if it is directly contained in this DirectoryEntry
    ///
    /// # Errors
    /// Returns `io::Error` with `io::ErrorKind::NotFound` if no Entry with the
    /// specified name exists in this DirectoryEntry
    fn get_entry_case_insensitive(&self, name: &str) -> io::Result<Box<dyn Entry>>;

    /// Create a new DocumentEntry
    ///
    /// # Arguments
    /// * `name` - the name of the new DocumentEntry
    /// * `stream` - the InputStream from which to create the new DocumentEntry
    ///
    /// # Returns
    /// The new DocumentEntry
    fn create_document(
        &mut self,
        name: &str,
        stream: &mut dyn io::Read,
    ) -> io::Result<Box<dyn DocumentEntry>>;

    /// Create a new DocumentEntry; the data will be provided later
    ///
    /// # Arguments
    /// * `name` - the name of the new DocumentEntry
    /// * `size` - the size of the new DocumentEntry
    /// * `writer` - the writer of the new DocumentEntry
    ///
    /// # Returns
    /// The new DocumentEntry
    fn create_document_with_writer(
        &mut self,
        name: &str,
        size: usize,
        writer: Box<dyn POIFSWriterListener>,
    ) -> io::Result<Box<dyn DocumentEntry>>;

    /// Create a new DirectoryEntry
    ///
    /// # Arguments
    /// * `name` - the name of the new DirectoryEntry
    ///
    /// # Returns
    /// The new DirectoryEntry
    fn create_directory(&mut self, name: &str) -> io::Result<Box<dyn DirectoryEntry>>;

    /// Gets the storage clsid of the directory entry
    ///
    /// # Returns
    /// storage TypeId
    fn storage_tid(&self) -> TypeId;

    /// Sets the storage clsid for the directory entry
    fn set_storage_tid(&mut self, tid_storage: TypeId);
}

/// This interface provides access to an object managed by a Filesystem instance.
/// Entry objects are further divided into DocumentEntry and DirectoryEntry instances.
pub trait Entry {
    /// Get the name of the Entry
    ///
    /// # Returns
    /// * Name of the entry
    fn get_name(&self) -> &str;

    /// Check if this is a DirectoryEntry
    ///
    /// # Returns
    /// * `true` if the Entry is a DirectoryEntry, else `false`
    fn is_directory_entry(&self) -> bool;

    /// Check if this is a DocumentEntry
    ///
    /// # Returns
    /// * `true` if the Entry is a DocumentEntry, else `false`
    fn is_document_entry(&self) -> bool;

    /// Get this Entry's parent (the DirectoryEntry that owns this Entry).
    /// All Entry objects, except the root Entry, have a parent.
    ///
    /// # Returns
    /// * This Entry's parent; `None` if this is the root Entry
    fn get_parent(&self) -> Option<&dyn DirectoryEntry>;

    /// Delete this Entry. This operation should succeed, but there are
    /// special circumstances when it will not:
    ///
    /// If this Entry is the root of the Entry tree, it cannot be
    /// deleted, as there is no way to create another one.
    ///
    /// If this Entry is a directory, it cannot be deleted unless it is
    /// empty.
    ///
    /// # Returns
    /// * `true` if the Entry was successfully deleted, else `false`
    fn delete(&mut self) -> bool;

    /// Rename this Entry. This operation will fail if:
    ///
    /// There is a sibling Entry (i.e., an Entry whose parent is the
    /// same as this Entry's parent) with the same name.
    ///
    /// This Entry is the root of the Entry tree. Its name is dictated
    /// by the Filesystem and many not be changed.
    ///
    /// # Arguments
    /// * `new_name` - The new name for this Entry
    ///
    /// # Returns
    /// * `true` if the operation succeeded, else `false`
    fn rename_to(&mut self, new_name: &str) -> bool;
}
