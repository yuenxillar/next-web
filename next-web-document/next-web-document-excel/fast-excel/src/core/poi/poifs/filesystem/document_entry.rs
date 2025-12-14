use crate::core::poi::ss::usermodel::directory_entry::Entry;

/// This interface defines methods specific to Document objects
/// managed by a Filesystem instance.
pub trait DocumentEntry: Entry {
    /// Get the size of the document, in bytes
    ///
    /// # Returns
    /// * Size in bytes
    fn get_size(&self) -> usize;
}
