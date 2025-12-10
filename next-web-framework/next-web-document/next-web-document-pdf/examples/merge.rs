use lopdf::Document;
use next_web_document_pdf::{operation::PdfOperation, operations::merge::PdfMergeOperation};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = format!("{}/assets", std::env::var("CARGO_MANIFEST_DIR")?);

    let docs = (2..5)
        .into_iter()
        .map(|index| Document::load(format!("{}/test_pdf_{}.pdf", path, index)))
        .collect::<Result<Vec<Document>, _>>()?;

    let oper = PdfMergeOperation { docs };

    let mut doc = Document::load(format!("{}/test_pdf_1.pdf", path))?;
    oper.execute(&mut doc)?.save(format!("{}/end.pdf", path))?;

    Ok(())
}
