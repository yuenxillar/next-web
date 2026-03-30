pub mod error;
pub mod operation;
pub mod operations;
pub mod processor;
pub mod signature;
pub mod util;
pub mod watermark;

pub type PdfResult<T> = std::result::Result<T, crate::error::pdf_error::PdfError>;

pub use operation::{PdfOperation, PdfQuery};
pub use processor::PdfProcessor;
#[cfg(feature = "word-convert")]
pub use operations::{
    pdf_to_word::{EmptyPageBehavior, PdfToWordOptions, WordDocument},
    word_to_pdf::{PdfPageSize, WordInputFormat, WordToPdfOptions},
};

// fn main() -> anyhow::Result<()> {
//     let mut pdf = PdfProcessor::from_file("input.pdf")?;

//     pdf.merge(&["a.pdf", "b.pdf"])?
//        .split_every(10, "output_part_{}.pdf")?
//        .rotate_pages(&[3, 5], 90)?
//        .add_text_watermark("机密文件", 48.0, 45.0)?
//        .add_image_watermark("logo.png", 0.3)?
//        .redact_region(1, 100.0, 100.0, 200.0, 50.0)?  // 真删除
//        .compress()?
//        .extract_text()?
//        .pdf_to_images(300)?.save_all("page_{}.png")?
//        .ocr_all_pages("chi_sim+eng")?
//        .fill_form(&[("name", "张三"), ("date", "2025-12-02")])?
//        .add_visible_signature("signature.png", 1, 400.0, 50.0, "张三 2025-12-02")?
//        .save("output_final.pdf")?;

//     Ok(())
// }
