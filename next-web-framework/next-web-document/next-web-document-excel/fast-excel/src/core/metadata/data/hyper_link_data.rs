use std::ops::{Deref, DerefMut};

use crate::core::metadata::data::coordinate_data::CoordinateData;

#[derive(Debug, Clone, Default)]
pub struct HyperlinkData {
    /// Depending on the hyperlink type it can be URL, e-mail, path to a file, etc
    address: Option<String>,
    /// hyperlink type
    hyperlink_type: Option<HyperlinkType>,

    coordinate_data: CoordinateData,
}

impl HyperlinkData {
    pub fn get_address(&self) -> Option<&str> {
        self.address.as_deref()
    }

    pub fn get_hyperlink_type(&self) -> Option<&HyperlinkType> {
        self.hyperlink_type.as_ref()
    }

    pub fn set_address(&mut self, address: String) {
        self.address = Some(address);
    }

    pub fn set_hyperlink_type(&mut self, hyperlink_type: HyperlinkType) {
        self.hyperlink_type = Some(hyperlink_type);
    }

    pub fn clear_address(&mut self) {
        self.address = None;
    }

    pub fn clear_hyperlink_type(&mut self) {
        self.hyperlink_type = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyperlinkType {
    /// Not a hyperlink
    None,
    /// Link to an existing file or web page
    Url,
    /// Link to a place in this document
    Document,
    /// Link to an E-mail address
    Email,
    /// Link to a file
    File,
}

impl Deref for HyperlinkData {
    type Target = CoordinateData;

    fn deref(&self) -> &Self::Target {
        &self.coordinate_data
    }
}

impl DerefMut for HyperlinkData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.coordinate_data
    }
}
