use crate::core::poi::ss::usermodel::{
    directory_entry::DirectoryEntry, picture_data::PictureData, simple_shape::SimpleShape,
};

/// Common interface for OLE shapes, i.e. shapes linked to embedded documents
pub trait ObjectData: SimpleShape {
    /// Gets the data portion, for an ObjectData that doesn't have an associated POIFS Directory Entry
    ///
    /// # Returns
    /// The object data as bytes
    fn object_data(&self) -> std::io::Result<Vec<u8>>;

    /// Checks if this ObjectData has an associated POIFS Directory Entry
    ///
    /// # Returns
    /// `true` if it has a directory entry, `false` if it only has a data portion
    fn has_directory_entry(&self) -> bool;

    /// Gets the object data as an OLE2 directory.
    ///
    /// # Note
    /// Only call for objects that have data though. See `has_directory_entry()`.
    /// The caller is responsible for closing the corresponding POIFSFileSystem.
    ///
    /// # Returns
    /// The object data as an OLE2 directory
    fn directory(&self) -> std::io::Result<Box<dyn DirectoryEntry>>;

    /// Gets the OLE2 Class Name of the object
    fn ole2_class_name(&self) -> String;

    /// Gets a filename suggestion - inspecting/interpreting the Directory object probably gives a better result
    fn file_name(&self) -> String;

    /// Gets the preview picture
    fn picture_data(&self) -> Option<&dyn PictureData>;

    /// Gets the content type
    fn content_type(&self) -> &str {
        "binary/octet-stream"
    }
}
