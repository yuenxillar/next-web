use crate::core::poi::ss::usermodel::{
    client_anchor::ClientAnchor, picture_data::PictureData, shape::Shape, sheet::Sheet,
};
use std::fmt;

/// Represents a picture in a SpreadsheetML document
pub trait Picture: Shape {
    /// Reset the image to the dimension of the embedded image
    ///
    /// # See Also
    /// `resize_with_scales()`
    fn resize(&mut self);

    /// Resize the image proportionally.
    ///
    /// # See Also
    /// `resize_with_scales()`
    fn resize_proportional(&mut self, scale: f64);

    /// Resize the image.
    ///
    /// # Note
    /// Please note, that this method works correctly only for workbooks
    /// with the default font size (Arial 10pt for .xls and Calibri 11pt for .xlsx).
    /// If the default font is changed the resized image can be stretched vertically or horizontally.
    ///
    /// # Examples
    /// * `resize_with_scales(1.0, 1.0)` keeps the original size
    /// * `resize_with_scales(0.5, 0.5)` resize to 50% of the original
    /// * `resize_with_scales(2.0, 2.0)` resizes to 200% of the original
    /// * `resize_with_scales(f64::MAX, f64::MAX)` resizes to the dimension of the embedded image
    ///
    /// # Arguments
    /// * `scale_x` - the amount by which the image width is multiplied relative to the original width.
    /// * `scale_y` - the amount by which the image height is multiplied relative to the original height.
    fn resize_with_scales(&mut self, scale_x: f64, scale_y: f64);

    /// Calculate the preferred size for this picture.
    ///
    /// # Returns
    /// ClientAnchor with the preferred size for this image
    fn preferred_size(&self) -> &dyn ClientAnchor;

    /// Calculate the preferred size for this picture.
    ///
    /// # Arguments
    /// * `scale_x` - the amount by which image width is multiplied relative to the original width.
    /// * `scale_y` - the amount by which image height is multiplied relative to the original height.
    ///
    /// # Returns
    /// ClientAnchor with the preferred size for this image
    fn preferred_size_with_scales(&self, scale_x: f64, scale_y: f64) -> &dyn ClientAnchor;

    /// Return the dimension of the embedded image in pixel
    ///
    /// # Returns
    /// image dimension in pixels
    fn image_dimension(&self) -> Dimension;

    /// Return picture data for this picture
    ///
    /// # Returns
    /// picture data for this picture
    fn picture_data(&self) -> Option<&dyn PictureData>;

    /// Returns the anchor that is used by this picture
    fn client_anchor(&self) -> &dyn ClientAnchor;

    /// Returns the sheet which contains the picture
    fn sheet(&self) -> &dyn Sheet;
}

/// The `Dimension` struct encapsulates the width and height of a component
/// (in integer precision) in a single object.
///
/// Normally the values of `width` and `height` are non-negative integers.
/// The constructors that allow you to create a dimension do
/// not prevent you from setting a negative value for these properties.
/// If the value of `width` or `height` is negative, the behavior of
/// some methods defined by other objects is undefined.
#[derive(Debug, Clone, Copy, Default)]
pub struct Dimension {
    /// The width dimension; negative values can be used.
    pub width: i32,

    /// The height dimension; negative values can be used.
    pub height: i32,
}

impl Dimension {
    /// Creates an instance of `Dimension` with a width of zero and a height of zero.
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }

    /// Creates an instance of `Dimension` from another dimension.
    ///
    /// # Arguments
    /// * `other` - Another dimension to copy width and height from
    pub fn from_dimension(other: &Dimension) -> Self {
        Self {
            width: other.width,
            height: other.height,
        }
    }

    /// Creates an instance of `Dimension` with the specified width and height.
    ///
    /// # Arguments
    /// * `width` - The width value
    /// * `height` - The height value
    pub fn with_size(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    /// Gets the width as a floating-point value.
    ///
    /// # Returns
    /// * Width as `f64`
    pub fn get_width(&self) -> f64 {
        self.width as f64
    }

    /// Gets the height as a floating-point value.
    ///
    /// # Returns
    /// * Height as `f64`
    pub fn get_height(&self) -> f64 {
        self.height as f64
    }

    /// Sets the size of this `Dimension` object to the specified width and height
    /// in double precision.
    ///
    /// Note that if `width` or `height` are larger than `i32::MAX`, they will
    /// be reset to `i32::MAX`.
    ///
    /// # Arguments
    /// * `width` - The new width for the `Dimension` object
    /// * `height` - The new height for the `Dimension` object
    pub fn set_size_double(&mut self, width: f64, height: f64) {
        let clamped_width = width.ceil().min(i32::MAX as f64).max(i32::MIN as f64) as i32;
        let clamped_height = height.ceil().min(i32::MAX as f64).max(i32::MIN as f64) as i32;

        self.width = clamped_width;
        self.height = clamped_height;
    }

    /// Gets a copy of the size of this `Dimension` object.
    ///
    /// # Returns
    /// * A new `Dimension` with the same width and height
    pub fn get_size(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
        }
    }

    /// Sets the size of this `Dimension` object to the size of another dimension.
    ///
    /// # Arguments
    /// * `other` - The new size for this `Dimension` object
    pub fn set_size_from_dimension(&mut self, other: &Dimension) {
        self.width = other.width;
        self.height = other.height;
    }

    /// Sets the size of this `Dimension` object to the specified width and height.
    ///
    /// # Arguments
    /// * `width` - The new width for this `Dimension` object
    /// * `height` - The new height for this `Dimension` object
    pub fn set_size(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
    }
}

impl PartialEq for Dimension {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width && self.height == other.height
    }
}

impl Eq for Dimension {}

impl std::hash::Hash for Dimension {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let sum = self.width as i64 + self.height as i64;
        let hash_code = sum * (sum + 1) / 2 + self.width as i64;
        hash_code.hash(state);
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dimension[width={}, height={}]", self.width, self.height)
    }
}
