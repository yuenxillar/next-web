use lopdf::Document;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::{
    PdfResult,
    operation::{PdfOperation, PdfQuery},
    operations::{
        compress::PdfCompressOperation,
        decrypt::PdfDecryptOperation,
        encrypt::PdfEncryptOperation,
        extract_text::PdfExtractTextOperation,
        merge::PdfMergeOperation,
        rotate::PdfRotateOperation,
        signature::{
            DigitalSignOptions, PdfDigitalSignOperation, PdfVisibleSignatureOperation,
            VisibleSignatureOptions,
        },
        split::{PdfSplitOperation, SplitType},
        watermark::{PdfWatermarkOperation, WatermarkLayout, WatermarkType},
    },
};
#[cfg(feature = "word-convert")]
use crate::operations::{
    pdf_to_word::{PdfToWordOperation, PdfToWordOptions, WordDocument},
    word_to_pdf::{WordToPdfOperation, WordToPdfOptions},
};

/// 以后所有功能都挂在这里的最终高层入口
pub struct PdfProcessor {
    pub(crate) doc: Document,
}

impl PdfProcessor {
    pub fn load<P: AsRef<Path>>(path: P) -> PdfResult<Self> {
        let doc = Document::load(path)?;
        Ok(Self { doc })
    }

    pub fn from_bytes(bytes: &[u8]) -> PdfResult<Self> {
        let doc = Document::load_from(bytes)?;
        Ok(Self { doc })
    }

    pub fn from_document(doc: Document) -> Self {
        Self { doc }
    }

    pub fn document(&self) -> &Document {
        &self.doc
    }

    pub fn into_document(self) -> Document {
        self.doc
    }

    pub fn merge_files<P: AsRef<Path>>(
        self,
        paths: impl IntoIterator<Item = P>,
    ) -> PdfResult<Self> {
        let docs = paths
            .into_iter()
            .map(Document::load)
            .collect::<Result<Vec<_>, _>>()?;
        self.merge_docs(docs)
    }

    pub fn merge_docs(mut self, docs: Vec<Document>) -> PdfResult<Self> {
        self.doc = PdfMergeOperation { docs }.execute(&mut self.doc)?;
        Ok(self)
    }

    pub fn split(&self, split_type: SplitType) -> PdfResult<Vec<Self>> {
        let mut cloned = self.doc.clone();
        let docs = PdfSplitOperation { split_type }.execute(&mut cloned)?;
        Ok(docs.into_iter().map(Self::from_document).collect())
    }

    pub fn rotate_all(mut self, angle: i64) -> PdfResult<Self> {
        PdfRotateOperation::with_angle(angle).execute(&mut self.doc)?;
        Ok(self)
    }

    pub fn rotate_pages<I>(mut self, pages: I, angle: i64) -> PdfResult<Self>
    where
        I: IntoIterator<Item = u32>,
    {
        let pages = pages.into_iter().collect::<HashSet<_>>();
        PdfRotateOperation::new(angle, pages).execute(&mut self.doc)?;
        Ok(self)
    }

    pub fn encrypt<T: Into<String>>(mut self, password: T) -> PdfResult<Self> {
        PdfEncryptOperation::new(password).execute(&mut self.doc)?;
        Ok(self)
    }

    pub fn encrypt_with<T: Into<String>, U: Into<String>>(
        mut self,
        owner_password: T,
        user_password: U,
    ) -> PdfResult<Self> {
        PdfEncryptOperation::with_pairs(owner_password, user_password).execute(&mut self.doc)?;
        Ok(self)
    }

    pub fn decrypt<T: Into<String>>(mut self, password: T) -> PdfResult<Self> {
        PdfDecryptOperation::new(password).execute(&mut self.doc)?;
        Ok(self)
    }

    pub fn compress(mut self) -> PdfResult<Self> {
        PdfCompressOperation::new(true).execute(&mut self.doc)?;
        Ok(self)
    }

    pub fn watermark_text<T, F>(
        mut self,
        content: T,
        font_name: Option<F>,
        font_size: f32,
        color_rgb: (f32, f32, f32),
        layout: WatermarkLayout,
        angle: f64,
        opacity: f32,
    ) -> PdfResult<Self>
    where
        T: Into<String>,
        F: Into<String>,
    {
        PdfWatermarkOperation {
            content: WatermarkType::Text {
                content: content.into(),
                font_name: font_name.map(Into::into),
                font_path: None,
                font_size,
                color_rgb,
            },
            layout,
            angle,
            opacity,
            offset: (0.0, 0.0),
        }
        .execute(&mut self.doc)?;
        Ok(self)
    }

    pub fn watermark_text_with_font_file<T, P>(
        mut self,
        content: T,
        font_path: P,
        font_size: f32,
        color_rgb: (f32, f32, f32),
        layout: WatermarkLayout,
        angle: f64,
        opacity: f32,
    ) -> PdfResult<Self>
    where
        T: Into<String>,
        P: Into<PathBuf>,
    {
        PdfWatermarkOperation {
            content: WatermarkType::Text {
                content: content.into(),
                font_name: None,
                font_path: Some(font_path.into()),
                font_size,
                color_rgb,
            },
            layout,
            angle,
            opacity,
            offset: (0.0, 0.0),
        }
        .execute(&mut self.doc)?;
        Ok(self)
    }

    pub fn watermark_text_at<T, F>(
        self,
        content: T,
        font_name: Option<F>,
        font_size: f32,
        color_rgb: (f32, f32, f32),
        x: f64,
        y: f64,
        angle: f64,
        opacity: f32,
    ) -> PdfResult<Self>
    where
        T: Into<String>,
        F: Into<String>,
    {
        self.watermark_text(
            content,
            font_name,
            font_size,
            color_rgb,
            WatermarkLayout::at(x, y),
            angle,
            opacity,
        )
    }

    pub fn watermark_text_with_font_file_at<T, P>(
        self,
        content: T,
        font_path: P,
        font_size: f32,
        color_rgb: (f32, f32, f32),
        x: f64,
        y: f64,
        angle: f64,
        opacity: f32,
    ) -> PdfResult<Self>
    where
        T: Into<String>,
        P: Into<PathBuf>,
    {
        self.watermark_text_with_font_file(
            content,
            font_path,
            font_size,
            color_rgb,
            WatermarkLayout::at(x, y),
            angle,
            opacity,
        )
    }

    pub fn watermark_image<P: Into<PathBuf>>(
        mut self,
        path: P,
        width: f32,
        height: f32,
        layout: WatermarkLayout,
        angle: f64,
        opacity: f32,
    ) -> PdfResult<Self> {
        PdfWatermarkOperation {
            content: WatermarkType::Image {
                path: path.into(),
                width,
                height,
            },
            layout,
            angle,
            opacity,
            offset: (0.0, 0.0),
        }
        .execute(&mut self.doc)?;
        Ok(self)
    }

    pub fn watermark_image_at<P: Into<PathBuf>>(
        self,
        path: P,
        width: f32,
        height: f32,
        x: f64,
        y: f64,
        angle: f64,
        opacity: f32,
    ) -> PdfResult<Self> {
        self.watermark_image(
            path,
            width,
            height,
            WatermarkLayout::at(x, y),
            angle,
            opacity,
        )
    }

    pub fn add_visible_signature(mut self, options: VisibleSignatureOptions) -> PdfResult<Self> {
        PdfVisibleSignatureOperation::new(options).execute(&mut self.doc)?;
        Ok(self)
    }

    pub fn extract_text(&self) -> PdfResult<String> {
        PdfExtractTextOperation::all_pages().query(&self.doc)
    }

    pub fn extract_text_from_pages<I>(&self, pages: I) -> PdfResult<String>
    where
        I: IntoIterator<Item = u32>,
    {
        PdfExtractTextOperation::from_pages(pages).query(&self.doc)
    }

    #[cfg(feature = "render")]
    pub fn pdf_to_images(
        &self,
        options: crate::operations::pdf_to_images::PdfToImagesOptions,
    ) -> PdfResult<crate::operations::pdf_to_images::PdfImageSet> {
        crate::operations::pdf_to_images::PdfToImagesOperation::new(options).query(&self.doc)
    }

    #[cfg(feature = "ocr")]
    pub fn ocr(
        &self,
        options: crate::operations::ocr::OcrOptions,
    ) -> PdfResult<crate::operations::ocr::OcrDocument> {
        crate::operations::ocr::PdfOcrOperation::new(options).query(&self.doc)
    }

    pub fn digital_sign(
        mut self,
        options: DigitalSignOptions,
    ) -> PdfResult<crate::operations::signature::SignedPdf> {
        PdfDigitalSignOperation::new(options).execute(&mut self.doc)
    }

    pub fn save<P: AsRef<Path>>(mut self, path: P) -> PdfResult<()> {
        self.doc.save(path)?;
        Ok(())
    }

    pub fn into_bytes(self) -> PdfResult<Vec<u8>> {
        crate::util::document_to_bytes(&self.doc)
    }

    pub fn page_count(&self) -> usize {
        self.doc.get_pages().len()
    }

    #[cfg(feature = "word-convert")]
    pub fn to_word(&self, options: PdfToWordOptions) -> PdfResult<WordDocument> {
        PdfToWordOperation::new(options).query(&self.doc)
    }

    #[cfg(feature = "word-convert")]
    pub fn from_word<P: AsRef<Path>>(path: P, options: WordToPdfOptions) -> PdfResult<Self> {
        let doc = WordToPdfOperation::from_path(path, options).convert()?;
        Ok(Self::from_document(doc))
    }

    #[cfg(feature = "word-convert")]
    pub fn from_word_bytes(bytes: &[u8], options: WordToPdfOptions) -> PdfResult<Self> {
        let doc = WordToPdfOperation::from_bytes(bytes, options).convert()?;
        Ok(Self::from_document(doc))
    }
}
