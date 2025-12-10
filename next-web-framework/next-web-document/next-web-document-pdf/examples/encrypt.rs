use lopdf::Document;
use next_web_document_pdf::{operation::PdfOperation, operations::encrypt::PdfEncryptOperation};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = format!("{}/assets", std::env::var("CARGO_MANIFEST_DIR")?);
    let mut doc = Document::load(format!("{}/example.pdf", path))?;

    let oper = PdfEncryptOperation::new("123456");

    oper.execute(&mut doc)?;

    doc.save(format!("{}/set_password.pdf", path))?;

    Ok(())
}
