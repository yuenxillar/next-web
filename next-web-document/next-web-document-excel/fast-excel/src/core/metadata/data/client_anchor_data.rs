use std::ops::{Deref, DerefMut};

use crate::core::metadata::data::coordinate_data::CoordinateData;

#[derive(Debug, Clone, Default)]
pub struct ClientAnchorData {
    top: Option<u32>,
    right: Option<u32>,
    bottom: Option<u32>,
    left: Option<u32>,
    anchor_type: Option<AnchorType>,

    coordinate_data: CoordinateData,
}

impl ClientAnchorData {
    pub fn get_top(&self) -> Option<u32> {
        self.top
    }

    pub fn get_right(&self) -> Option<u32> {
        self.right
    }

    pub fn get_bottom(&self) -> Option<u32> {
        self.bottom
    }

    pub fn get_left(&self) -> Option<u32> {
        self.left
    }

    pub fn get_anchor_type(&self) -> Option<&AnchorType> {
        self.anchor_type.as_ref()
    }

    pub fn set_top(&mut self, top: u32) {
        self.top = Some(top);
    }

    pub fn set_right(&mut self, right: u32) {
        self.right = Some(right);
    }

    pub fn set_bottom(&mut self, bottom: u32) {
        self.bottom = Some(bottom);
    }

    pub fn set_left(&mut self, left: u32) {
        self.left = Some(left);
    }

    pub fn set_anchor_type(&mut self, anchor_type: AnchorType) {
        self.anchor_type = Some(anchor_type);
    }

    pub fn clear_top(&mut self) {
        self.top = None;
    }

    pub fn clear_right(&mut self) {
        self.right = None;
    }

    pub fn clear_bottom(&mut self) {
        self.bottom = None;
    }

    pub fn clear_left(&mut self) {
        self.left = None;
    }

    pub fn clear_anchor_type(&mut self) {
        self.anchor_type = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorType {
    MoveAndResize,
    DontMoveDoResize,
    MoveDontResize,
    DontMoveAndResize,
}

impl Deref for ClientAnchorData {
    type Target = CoordinateData;

    fn deref(&self) -> &Self::Target {
        &self.coordinate_data
    }
}

impl DerefMut for ClientAnchorData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.coordinate_data
    }
}
