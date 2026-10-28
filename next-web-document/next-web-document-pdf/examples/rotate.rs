use lopdf::Document;
use next_web_document_pdf::{operation::PdfOperation, operations::rotate::PdfRotateOperation};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = format!("{}/assets", std::env::var("CARGO_MANIFEST_DIR")?);

    let oper = PdfRotateOperation::new(-90, [1, 2]);

    let mut doc = Document::load(format!("{}/end.pdf", path))?;
    oper.execute(&mut doc)?;

    doc.save(format!("{}/rotate.pdf", path))?;

    Ok(())
}
