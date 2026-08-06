use std::ops::Deref;

use crate::util::MimeType;

pub struct MimeTypeUtils;

impl MimeTypeUtils {
    pub fn tokenize(s: &str) -> Vec<&str> {
        todo!()
    }

    pub fn to_string(mime_types: Vec<&MimeType>) -> String {
        todo!()
    }

    pub fn parse_mime_type(mime_type: &str) -> MimeType {
        todo!()
    }

    pub fn sort_by_specificity<T>(mime_types: &mut Vec<T>) -> Result<(), &'static str>
    where
        T: Deref<Target = MimeType>,
    {
        todo!()
    }
}
