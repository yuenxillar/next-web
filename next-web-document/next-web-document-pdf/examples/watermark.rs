use lopdf::Document;
use next_web_document_pdf::{
    operation::PdfOperation,
    operations::watermark::{PdfWatermarkOperation, WatermarkLayout, WatermarkType},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = format!("{}/assets", std::env::var("CARGO_MANIFEST_DIR")?);
    let operation = PdfWatermarkOperation {
        content: WatermarkType::Text {
            content: "Hello World".into(),

            font_name: Some("Helvetica".into()),
            font_size: 22.0,
            color_rgb: (0.5, 0.5, 0.5),
        },
        layout: WatermarkLayout::Tile {
            gap_x: 10.0,
            gap_y: 10.0,
            stagger: true,
        },
        angle: -45.0,
        opacity: 0.3,
        offset: (-30.0, 0.0),
    };
    let mut doc = Document::load(format!("{}/end.pdf", path))?;
    operation.execute(&mut doc)?;

    doc.save(format!("{}/watermarked.pdf", path))?;
    Ok(())
}
