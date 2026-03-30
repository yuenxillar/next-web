use lopdf::Document;
use std::{fs, process::Command};

use crate::{PdfResult, error::pdf_error::PdfError, operation::PdfQuery};

#[derive(Debug, Clone)]
pub struct OcrOptions {
    pub language: String,
    pub dpi: u16,
}

impl Default for OcrOptions {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            dpi: 150,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OcrPage {
    pub page_number: usize,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct OcrDocument {
    pages: Vec<OcrPage>,
}

impl OcrDocument {
    pub fn new(pages: Vec<OcrPage>) -> Self {
        Self { pages }
    }

    pub fn pages(&self) -> &[OcrPage] {
        &self.pages
    }

    pub fn full_text(&self) -> String {
        self.pages
            .iter()
            .map(|page| page.text.trim_end())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

pub struct PdfOcrOperation {
    options: OcrOptions,
}

impl PdfOcrOperation {
    pub fn new(options: OcrOptions) -> Self {
        Self { options }
    }
}

impl PdfQuery<OcrDocument> for PdfOcrOperation {
    fn query(&self, doc: &Document) -> PdfResult<OcrDocument> {
        let images = crate::operations::pdf_to_images::PdfToImagesOperation::new(
            crate::operations::pdf_to_images::PdfToImagesOptions {
                dpi: self.options.dpi,
            },
        )
        .query(doc)?;

        let mut pages = Vec::with_capacity(images.len());
        for (index, image) in images.images().iter().enumerate() {
            let text = ocr_image_with_tesseract_cli(
                image,
                &self.options.language,
                self.options.dpi,
                index + 1,
            )?;

            pages.push(OcrPage {
                page_number: index + 1,
                text,
            });
        }

        Ok(OcrDocument::new(pages))
    }
}

fn ocr_image_with_tesseract_cli(
    image: &image::DynamicImage,
    language: &str,
    dpi: u16,
    page_number: usize,
) -> PdfResult<String> {
    let file_name = format!(
        "next-web-pdf-ocr-{}-{}-{}.png",
        std::process::id(),
        page_number,
        rand::random::<u64>()
    );
    let temp_path = std::env::temp_dir().join(file_name);

    image.save_with_format(&temp_path, image::ImageFormat::Png).map_err(|error| {
        PdfError::OcrError(format!(
            "failed to save temporary OCR image for page {page_number}: {error}"
        ))
    })?;

    let output = Command::new("tesseract")
        .arg(&temp_path)
        .arg("stdout")
        .arg("-l")
        .arg(language)
        .arg("--dpi")
        .arg(dpi.to_string())
        .output()
        .map_err(|error| {
            PdfError::RuntimeDependencyMissing(format!(
                "failed to launch tesseract executable: {error}"
            ))
        })?;

    let _ = fs::remove_file(&temp_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PdfError::OcrError(format!(
            "tesseract failed on page {page_number}: {}",
            stderr.trim()
        )));
    }

    String::from_utf8(output.stdout).map_err(|error| {
        PdfError::OcrError(format!(
            "tesseract returned non-UTF8 output for page {page_number}: {error}"
        ))
    })
}
