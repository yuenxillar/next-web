use std::{fs, path::Path};

use lopdf::Document;

use crate::PdfResult;

pub fn document_to_bytes(doc: &Document) -> PdfResult<Vec<u8>> {
    let mut cloned = doc.clone();
    let mut bytes = Vec::new();
    cloned.save_to(&mut bytes)?;
    Ok(bytes)
}

pub fn save_bytes<P: AsRef<Path>>(path: P, bytes: &[u8]) -> PdfResult<()> {
    fs::write(path, bytes)?;
    Ok(())
}

pub fn write_dynamic_image(
    image: &image::DynamicImage,
    format: image::ImageFormat,
) -> PdfResult<Vec<u8>> {
    let mut bytes = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut bytes), format)
        .map_err(|error| crate::error::pdf_error::PdfError::Custom(error.to_string()))?;
    Ok(bytes)
}
