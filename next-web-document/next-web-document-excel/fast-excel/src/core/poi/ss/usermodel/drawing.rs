use crate::core::poi::ss::usermodel::{
    client_anchor::ClientAnchor, comment::Comment, object_data::ObjectData, picture::Picture,
    shape::Shape, shape_container::ShapeContainer,
};

/// High level representation of spreadsheet drawing.
pub trait Drawing<T: Shape>: ShapeContainer<T> {
    /// Creates a picture.
    ///
    /// # Arguments
    /// * `anchor` - the client anchor describes how this picture is attached to the sheet.
    /// * `picture_index` - the index of the picture in the workbook collection of pictures.
    ///
    /// # Returns
    /// The newly created picture.
    fn create_picture(&mut self, anchor: &dyn ClientAnchor, picture_index: i32)
    -> Box<dyn Picture>;

    /// Creates a comment.
    ///
    /// # Arguments
    /// * `anchor` - the client anchor describes how this comment is attached to the sheet.
    ///
    /// # Returns
    /// The newly created comment.
    fn create_cell_comment(&mut self, anchor: &dyn ClientAnchor) -> Box<dyn Comment>;

    /// Creates a new client anchor and sets the top-left and bottom-right coordinates of the anchor.
    ///
    /// # Arguments
    /// * `dx1` - the x coordinate in EMU within the first cell.
    /// * `dy1` - the y coordinate in EMU within the first cell.
    /// * `dx2` - the x coordinate in EMU within the second cell.
    /// * `dy2` - the y coordinate in EMU within the second cell.
    /// * `col1` - the column (0 based) of the first cell.
    /// * `row1` - the row (0 based) of the first cell.
    /// * `col2` - the column (0 based) of the second cell.
    /// * `row2` - the row (0 based) of the second cell.
    ///
    /// # Returns
    /// The newly created client anchor
    fn create_anchor(
        &mut self,
        dx1: i32,
        dy1: i32,
        dx2: i32,
        dy2: i32,
        col1: i32,
        row1: i32,
        col2: i32,
        row2: i32,
    ) -> Box<dyn ClientAnchor>;

    /// Adds a new OLE Package Shape
    ///
    /// # Arguments
    /// * `anchor` - the client anchor describes how this picture is attached to the sheet.
    /// * `storage_id` - the storageId returned by `Workbook::add_ole_package()`
    /// * `picture_index` - the index of the picture (used as preview image) in the
    ///                     workbook collection of pictures.
    ///
    /// # Returns
    /// Newly created shape
    fn create_object_data(
        &mut self,
        anchor: &dyn ClientAnchor,
        storage_id: i32,
        picture_index: i32,
    ) -> Box<dyn ObjectData>;
}
