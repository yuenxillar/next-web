use std::borrow::Cow;
use std::io::{self};
use std::path::Path;

/// Interface for a resource descriptor that abstracts from the actual
/// type of underlying resource, such as a file or class path resource.
///
/// An `InputStream` can be opened for every resource if it exists in
/// physical form, but a URL or `File` handle can just be returned for
/// certain resources. The actual behavior is implementation-specific.
pub trait Resource {
    /// Return the path of this resource.
    ///
    /// Note: This only works for files in the default file system.
    ///
    /// Returns an error if the resource cannot be resolved as a file path,
    /// i.e. if the resource is not available as a file in a file system.
    fn path(&self) -> io::Result<&Path>;

    /// Return the contents of this resource as a byte array.
    ///
    /// Returns the contents of this resource as a byte array.
    ///
    /// Returns an error if the resource cannot be resolved as an absolute
    /// file path, i.e. if the resource is not available in a file system,
    /// or in case of general resolution/reading failures.
    fn get_content(&self) -> io::Result<Cow<'static, [u8]>>;

    /// Return the contents of this resource as a string, using the specified charset.
    ///
    /// Returns the contents of this resource as a `String`.
    ///
    /// Returns an error if the resource cannot be resolved as an absolute
    /// file path, i.e. if the resource is not available in a file system,
    /// or in case of general resolution/reading failures.
    fn get_content_as_string(&self) -> io::Result<String> {
        let bytes = self.get_content()?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Determine the content length for this resource.
    fn content_length(&self) -> u64;

    /// Determine the last-modified timestamp for this resource.
    fn last_modified(&self) -> Option<u64>;

    /// Determine the created timestamp for this resource.
    fn created(&self) -> Option<u64>;

    /// Create a resource relative to this resource.
    ///
    /// # Parameters
    ///
    /// - `relative_path`: the relative path (relative to this resource).
    ///
    /// Returns the resource handle for the relative resource.
    ///
    /// Returns an error if the relative resource cannot be determined.
    fn create_relative(&self, relative_path: &str) -> io::Result<Box<dyn Resource>>;

    /// Determine the filename for this resource — typically the last
    /// part of the path — for example, `"myfile.txt"`.
    ///
    /// Returns `None` if this type of resource does not
    /// have a filename.
    ///
    /// Implementations are encouraged to return the filename unencoded.
    fn filename(&self) -> Option<&str>;
}
