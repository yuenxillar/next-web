use lopdf::Document;
use next_web_document_pdf::{
    operation::PdfOperation,
    operations::watermark::{PdfWatermarkOperation, WatermarkLayout, WatermarkType},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = format!("{}/assets", std::env::var("CARGO_MANIFEST_DIR")?);
    let operation = PdfWatermarkOperation {
        content: WatermarkType::Text {
            content: "Watermark6666".into(),
            font_name: Some("Helvetica".into()),
            font_path: None,
            font_size: 48.0,
            color_rgb: (1.0, 0.35, 0.35),
        },
        layout: WatermarkLayout::centered(),
        angle: -45.0,
        opacity: 0.18,
        offset: (0.0, 0.0),
    };
    let mut doc = Document::load(format!("{}/end.pdf", path))?;
    operation.execute(&mut doc)?;

    doc.save(format!("{}/watermarked.pdf", path))?;
    Ok(())
}
