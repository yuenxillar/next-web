use image::{DynamicImage, ImageFormat};
use lopdf::Document;
use std::path::Path;

use crate::{
    PdfResult,
    error::pdf_error::PdfError,
    operation::PdfQuery,
    util::{document_to_bytes, write_dynamic_image},
};

use pdfium_render::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct PdfToImagesOptions {
    pub dpi: u16,
}

impl Default for PdfToImagesOptions {
    fn default() -> Self {
        Self { dpi: 150 }
    }
}

#[derive(Debug, Clone)]
pub struct PdfImageSet {
    images: Vec<DynamicImage>,
}

impl PdfImageSet {
    pub fn new(images: Vec<DynamicImage>) -> Self {
        Self { images }
    }

    pub fn len(&self) -> usize {
        self.images.len()
    }

    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    pub fn images(&self) -> &[DynamicImage] {
        &self.images
    }

    pub fn into_images(self) -> Vec<DynamicImage> {
        self.images
    }

    pub fn save_all<P: AsRef<str>>(&self, pattern: P) -> PdfResult<()> {
        let pattern = pattern.as_ref();
        for (index, image) in self.images.iter().enumerate() {
            let path = render_output_path(pattern, index + 1);
            image.save_with_format(&path, ImageFormat::Png).map_err(|error| {
                PdfError::RenderError(format!("failed to save page image to {path}: {error}"))
            })?;
        }
        Ok(())
    }
}

pub struct PdfToImagesOperation {
    options: PdfToImagesOptions,
}

impl PdfToImagesOperation {
    pub fn new(options: PdfToImagesOptions) -> Self {
        Self { options }
    }
}

impl PdfQuery<PdfImageSet> for PdfToImagesOperation {
    fn query(&self, doc: &Document) -> PdfResult<PdfImageSet> {
        let bytes = document_to_bytes(doc)?;
        let pdfium = bind_pdfium()?;
        let document = pdfium
            .load_pdf_from_byte_vec(bytes, None)
            .map_err(|error| PdfError::RenderError(format!("failed to open PDF in Pdfium: {error}")))?;

        let scale = f32::from(self.options.dpi) / 72.0;
        let render_config = PdfRenderConfig::new().scale_page_by_factor(scale.max(0.1));

        let mut images = Vec::new();
        for (index, page) in document.pages().iter().enumerate() {
            let image = page
                .render_with_config(&render_config)
                .map_err(|error| {
                    PdfError::RenderError(format!(
                        "failed to render PDF page {}: {error}",
                        index + 1
                    ))
                })?
                .as_image();
            images.push(image);
        }

        Ok(PdfImageSet::new(images))
    }
}

pub fn bind_pdfium() -> PdfResult<Pdfium> {
    let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
        .or_else(|_| Pdfium::bind_to_system_library())
        .map_err(|error| {
            PdfError::RuntimeDependencyMissing(format!(
                "unable to bind Pdfium runtime library: {error}"
            ))
        })?;
    Ok(Pdfium::new(bindings))
}

pub fn dynamic_image_to_png_bytes(image: &DynamicImage) -> PdfResult<Vec<u8>> {
    write_dynamic_image(image, ImageFormat::Png)
        .map_err(|error| PdfError::RenderError(format!("failed to encode page image: {error}")))
}

fn render_output_path(pattern: &str, page_number: usize) -> String {
    if pattern.contains("{}") {
        pattern.replacen("{}", &page_number.to_string(), 1)
    } else {
        let path = Path::new(pattern);
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("page");
        let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("png");
        let parent = path.parent().and_then(|parent| parent.to_str()).unwrap_or("");

        if parent.is_empty() {
            format!("{stem}_{page_number}.{ext}")
        } else {
            format!("{parent}\\{stem}_{page_number}.{ext}")
        }
    }
}
