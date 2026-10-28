use std::ops::{Deref, DerefMut};

use crate::core::metadata::data::client_anchor_data::ClientAnchorData;

#[derive(Debug, Clone, Default)]
pub struct ImageData {
    image: Option<Vec<u8>>,
    image_type: Option<ImageType>,

    client_anchor_data: ClientAnchorData,
}

impl ImageData {
    pub fn get_image(&self) -> Option<&Vec<u8>> {
        self.image.as_ref()
    }

    pub fn get_image_type(&self) -> Option<&ImageType> {
        self.image_type.as_ref()
    }

    pub fn set_image(&mut self, image: Vec<u8>) {
        self.image = Some(image);
    }

    pub fn set_image_type(&mut self, image_type: ImageType) {
        self.image_type = Some(image_type);
    }

    pub fn clear_image(&mut self) {
        self.image = None;
    }

    pub fn clear_image_type(&mut self) {
        self.image_type = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageType {
    Emf,
    Wmf,
    Pict,
    Jpeg,
    Png,
    Dib,
}

impl ImageType {
    pub fn value(&self) -> u8 {
        match self {
            ImageType::Emf => 2,
            ImageType::Wmf => 3,
            ImageType::Pict => 4,
            ImageType::Jpeg => 5,
            ImageType::Png => 6,
            ImageType::Dib => 7,
        }
    }
}

impl Deref for ImageData {
    type Target = ClientAnchorData;

    fn deref(&self) -> &Self::Target {
        &self.client_anchor_data
    }
}

impl DerefMut for ImageData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client_anchor_data
    }
}
