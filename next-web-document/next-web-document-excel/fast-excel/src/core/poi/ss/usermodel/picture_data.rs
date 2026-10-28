pub trait PictureData {
    /// Gets the picture data.
    ///
    /// # Returns
    /// The picture data as a byte slice.
    fn data(&self) -> &[u8];

    /// Suggests a file extension for this image.
    ///
    /// # Returns
    /// The file extension, without a leading `.`. Never `None`, but will be empty string if the extension
    /// is unknown or if the file backing these data does not have an extension.
    fn suggest_file_extension(&self) -> Option<&str>;

    /// Returns the mime type for the image
    fn mime_type(&self) -> Option<&str>;

    /// Returns the POI internal image type, `0` if unknown image type
    ///
    /// # See Also
    /// * `Workbook::PICTURE_TYPE_DIB`
    /// * `Workbook::PICTURE_TYPE_EMF`
    /// * `Workbook::PICTURE_TYPE_JPEG`
    /// * `Workbook::PICTURE_TYPE_PICT`
    /// * `Workbook::PICTURE_TYPE_PNG`
    /// * `Workbook::PICTURE_TYPE_WMF`
    fn picture_type(&self) -> i32;
}
