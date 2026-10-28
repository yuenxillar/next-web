use lopdf::Document;
use next_web_document_pdf::{
    operation::PdfOperation,
    operations::split::{PdfSplitOperation, SplitType},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = format!("{}/assets", std::env::var("CARGO_MANIFEST_DIR")?);
    let operation = PdfSplitOperation {
        split_type: SplitType::Single { start: 2, end: 4 },
    };
    let docs = operation.execute(&mut Document::load(format!("{}/end.pdf", path))?)?;
    for (index, mut doc) in docs.into_iter().enumerate() {
        doc.save(format!("split_{}.pdf", index))?;
    }
    Ok(())
}
