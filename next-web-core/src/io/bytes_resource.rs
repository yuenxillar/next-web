use std::{borrow::Cow, io, path::Path};

use crate::io::Resource;

/// A [`Resource`] backed by an in-memory byte buffer.
///
/// `BytesResource` is typically used for data that does not originate from a
/// real file on disk, such as resources embedded at compile time
/// (via [`include_bytes!`]) or dynamically generated content. The path is kept
/// purely for identification and display purposes; no filesystem access is
/// performed.
///
/// The stored data is a [`Cow<'static, [u8]>`], which allows callers to
/// provide either:
///
/// - a borrowed `&'static [u8]` (zero-copy, e.g. from `include_bytes!`), or
/// - an owned `Vec<u8>` that is moved into the resource.
///
/// Cloning the resource is cheap when the data is borrowed and requires a copy
/// when the data is owned.
///
/// # Examples
///
/// ```ignore
/// use std::borrow::Cow;
///
/// let resource = BytesResource::new(
///     Path::new("embedded/config.json"),
///     Cow::Borrowed(include_bytes!("config.json")),
/// );
/// ```
pub struct BytesResource<'a> {
    /// Logical path used to identify the resource.
    ///
    /// This path is returned by [`Resource::path`] and is not validated
    /// against any filesystem. It should be treated as an opaque identifier.
    path: &'a Path,

    /// In-memory contents of the resource.
    ///
    /// Stored as a [`Cow`] so that static data can be borrowed without copying
    /// and runtime-generated data can be owned.
    data: Cow<'static, [u8]>,
}

impl<'a> BytesResource<'a> {
    /// Creates a new [`BytesResource`] from a path and its in-memory contents.
    ///
    /// # Parameters
    ///
    /// - `path`: Logical path identifying the resource. Only
    ///   [`AsRef<Path>`] is required, so both `&Path` and `&str` (and other
    ///   path-like types) can be passed directly.
    /// - `data`: The resource contents. Use [`Cow::Borrowed`] for static data
    ///   to avoid copying, or [`Cow::Owned`] for dynamically produced data.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Borrowed, zero-copy.
    /// let a = BytesResource::new("a.txt", Cow::Borrowed(b"hello"));
    ///
    /// // Owned, moved into the resource.
    /// let b = BytesResource::new("b.txt", Cow::Owned(vec![1, 2, 3]));
    /// ```
    pub fn new<P>(path: &'a P, data: Cow<'static, [u8]>) -> Self
    where
        P: AsRef<Path>,
        P: ?Sized,
    {
        Self {
            path: path.as_ref(),
            data,
        }
    }
}

impl Resource for BytesResource<'_> {
    /// Returns the logical path of this resource.
    ///
    /// The returned path is the one supplied at construction time and does not
    /// necessarily correspond to a real file on disk.
    fn path(&self) -> io::Result<&Path> {
        Ok(self.path)
    }

    /// Returns the in-memory contents of this resource.
    ///
    /// The contents are returned as a [`Cow<'static, [u8]>`]. When the
    /// underlying buffer is borrowed, this is a zero-copy operation; when it is
    /// owned, the data is cloned.
    fn get_content(&self) -> io::Result<Cow<'static, [u8]>> {
        Ok(self.data.clone())
    }

    /// Returns the length of the resource contents, in bytes.
    ///
    /// This is computed from the in-memory buffer and never touches the
    /// filesystem.
    fn content_length(&self) -> u64 {
        self.data.len() as u64
    }

    /// Returns the last modification time, if available.
    ///
    /// `BytesResource` has no filesystem backing, so this always returns
    /// [`None`].
    fn last_modified(&self) -> Option<u64> {
        None
    }

    /// Returns the creation time, if available.
    ///
    /// `BytesResource` has no filesystem backing, so this always returns
    /// [`None`].
    fn created(&self) -> Option<u64> {
        None
    }

    /// Returns the file name component of the resource path, if any.
    ///
    /// Returns an empty string when the path has no file name component (for
    /// example, `"/"` or a path ending in `..`), and [`None`] only when the
    /// path cannot be resolved at all. This mirrors the behavior of
    /// [`Path::file_name`] combined with [`OsStr::to_str`].
    fn filename(&self) -> Option<&str> {
        self.path()
            .ok()
            .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or(""))
    }

    /// Attempts to resolve a relative resource.
    ///
    /// `BytesResource` is purely in-memory and has no notion of a surrounding
    /// directory, so relative resolution is not supported. This method always
    /// returns an [`io::ErrorKind::Unsupported`] error.
    ///
    /// # Errors
    ///
    /// Always returns [`io::ErrorKind::Unsupported`].
    #[allow(unused_variables)]
    fn create_relative(&self, relative_path: &str) -> io::Result<Box<dyn Resource>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "BytesResource does not support relative path resolution",
        ))
    }
}
