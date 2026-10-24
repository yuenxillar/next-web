/// Common interface for anchors.
///
/// An anchor is what specifics the position of a shape within a client object
/// or within another containing shape.
pub trait ChildAnchor: Send + Sync {
    /// Get the x coordinate of the left up corner
    ///
    /// # Returns
    /// * X coordinate of the left up corner
    fn get_dx1(&self) -> i32;

    /// Set the x coordinate of the left up corner
    ///
    /// # Arguments
    /// * `dx1` - X coordinate of the left up corner
    fn set_dx1(&mut self, dx1: i32);

    /// Get the y coordinate of the left up corner
    ///
    /// # Returns
    /// * Y coordinate of the left up corner
    fn get_dy1(&self) -> i32;

    /// Set the y coordinate of the left up corner
    ///
    /// # Arguments
    /// * `dy1` - Y coordinate of the left up corner
    fn set_dy1(&mut self, dy1: i32);

    /// Get the y coordinate of the right down corner
    ///
    /// # Returns
    /// * Y coordinate of the right down corner
    fn get_dy2(&self) -> i32;

    /// Set the y coordinate of the right down corner
    ///
    /// # Arguments
    /// * `dy2` - Y coordinate of the right down corner
    fn set_dy2(&mut self, dy2: i32);

    /// Get the x coordinate of the right down corner
    ///
    /// # Returns
    /// * X coordinate of the right down corner
    fn get_dx2(&self) -> i32;

    /// Set the x coordinate of the right down corner
    ///
    /// # Arguments
    /// * `dx2` - X coordinate of the right down corner
    fn set_dx2(&mut self, dx2: i32);
}
