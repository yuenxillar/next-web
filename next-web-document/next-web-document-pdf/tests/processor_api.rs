use std::path::PathBuf;

use next_web_document_pdf::{
    PdfProcessor,
    operations::{
        signature::{SignatureRect, VisibleSignatureOptions},
        split::SplitType,
        watermark::{WatermarkAnchor, WatermarkLayout},
    },
};
#[cfg(feature = "word-convert")]
use next_web_document_pdf::{
    EmptyPageBehavior, PdfToWordOptions, WordInputFormat, WordToPdfOptions,
};
#[cfg(feature = "word-convert")]
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

fn asset_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(name)
}

#[test]
fn processor_loads_and_counts_pages() {
    let processor = PdfProcessor::load(asset_path("test_pdf_1.pdf")).unwrap();
    assert!(processor.page_count() > 0);
}

#[test]
fn processor_chain_round_trips_bytes() {
    let processor = PdfProcessor::load(asset_path("test_pdf_1.pdf"))
        .unwrap()
        .rotate_all(90)
        .unwrap()
        .compress()
        .unwrap();

    let bytes = processor.into_bytes().unwrap();
    let reloaded = PdfProcessor::from_bytes(&bytes).unwrap();

    assert!(reloaded.page_count() > 0);
}

#[test]
fn processor_extracts_text_from_example_fixture() {
    let processor = PdfProcessor::load(asset_path("example.pdf")).unwrap();
    let text = processor.extract_text().unwrap();
    assert!(text.contains("Hello"));
}

#[test]
fn processor_split_preserves_total_page_count() {
    let processor = PdfProcessor::load(asset_path("test_pdf_1.pdf")).unwrap();
    let original_pages = processor.page_count();

    let parts = processor
        .split(SplitType::Single { start: 1, end: 1 })
        .unwrap();
    let split_total: usize = parts.iter().map(PdfProcessor::page_count).sum();

    assert!(!parts.is_empty());
    assert_eq!(split_total, original_pages);
}

#[test]
fn processor_adds_visible_signature_and_remains_loadable() {
    let bytes = PdfProcessor::load(asset_path("example.pdf"))
        .unwrap()
        .add_visible_signature(VisibleSignatureOptions {
            page: 1,
            rect: SignatureRect::new(100.0, 120.0, 160.0, 48.0),
            image_path: None,
            text: Some("Signed by next-web".to_string()),
            ..VisibleSignatureOptions::default()
        })
        .unwrap()
        .into_bytes()
        .unwrap();

    let reloaded = PdfProcessor::from_bytes(&bytes).unwrap();
    assert_eq!(reloaded.page_count(), 1);
}

#[test]
fn processor_applies_text_watermark_and_remains_loadable() {
    let source = PdfProcessor::load(asset_path("test_pdf_1.pdf")).unwrap();
    let expected_pages = source.page_count();

    let bytes = source
        .watermark_text(
            "CONFIDENTIAL",
            Some("Helvetica"),
            24.0,
            (0.7, 0.2, 0.2),
            WatermarkLayout::Single {
                offset_x: 0.0,
                offset_y: 0.0,
            },
            -30.0,
            0.35,
        )
        .unwrap()
        .into_bytes()
        .unwrap();

    let reloaded = PdfProcessor::from_bytes(&bytes).unwrap();
    assert_eq!(reloaded.page_count(), expected_pages);
}

#[test]
fn processor_applies_positioned_text_watermark_and_remains_loadable() {
    let bytes = PdfProcessor::load(asset_path("test_pdf_1.pdf"))
        .unwrap()
        .watermark_text_at(
            "APPROVED",
            Some("Helvetica"),
            18.0,
            (0.2, 0.4, 0.8),
            140.0,
            220.0,
            0.0,
            0.55,
        )
        .unwrap()
        .into_bytes()
        .unwrap();

    let reloaded = PdfProcessor::from_bytes(&bytes).unwrap();
    assert!(reloaded.page_count() > 0);
}

#[test]
fn processor_applies_anchored_text_watermark_and_remains_loadable() {
    let bytes = PdfProcessor::load(asset_path("test_pdf_1.pdf"))
        .unwrap()
        .watermark_text(
            "DRAFT",
            Some("Helvetica"),
            20.0,
            (0.8, 0.2, 0.2),
            WatermarkLayout::anchored(WatermarkAnchor::TopRight, -36.0, -28.0),
            0.0,
            0.4,
        )
        .unwrap()
        .into_bytes()
        .unwrap();

    let reloaded = PdfProcessor::from_bytes(&bytes).unwrap();
    assert!(reloaded.page_count() > 0);
}

#[test]
fn processor_applies_font_file_text_watermark_when_font_is_available() {
    let Some(font_path) = find_system_font() else {
        return;
    };

    let bytes = PdfProcessor::load(asset_path("test_pdf_1.pdf"))
        .unwrap()
        .watermark_text_with_font_file(
            "自定义字体",
            font_path,
            28.0,
            (0.1, 0.45, 0.75),
            WatermarkLayout::anchored(WatermarkAnchor::BottomCenter, 0.0, 42.0),
            0.0,
            0.5,
        )
        .unwrap()
        .into_bytes()
        .unwrap();

    let reloaded = PdfProcessor::from_bytes(&bytes).unwrap();
    assert!(reloaded.page_count() > 0);
}

#[cfg(feature = "render")]
#[test]
fn render_feature_returns_images_or_runtime_error() {
    let processor = PdfProcessor::load(asset_path("example.pdf")).unwrap();
    match processor.pdf_to_images(Default::default()) {
        Ok(images) => assert_eq!(images.len(), processor.page_count()),
        Err(error) => {
            let message = error.to_string();
            assert!(message.contains("Pdfium") || message.contains("pdfium"));
        }
    }
}

#[cfg(feature = "ocr")]
#[test]
fn ocr_feature_returns_text_or_runtime_error() {
    let processor = PdfProcessor::load(asset_path("example.pdf")).unwrap();
    match processor.ocr(Default::default()) {
        Ok(result) => assert!(!result.full_text().is_empty()),
        Err(error) => {
            let message = error.to_string();
            assert!(message.contains("tesseract") || message.contains("Pdfium") || message.contains("pdfium"));
        }
    }
}

fn find_system_font() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from(r"C:\Windows\Fonts\msyh.ttc"),
        PathBuf::from(r"C:\Windows\Fonts\msyh.ttf"),
        PathBuf::from(r"C:\Windows\Fonts\simhei.ttf"),
        PathBuf::from(r"C:\Windows\Fonts\arial.ttf"),
        PathBuf::from("/System/Library/Fonts/Supplemental/Arial Unicode.ttf"),
        PathBuf::from("/System/Library/Fonts/Supplemental/Arial.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
        PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"),
    ];

    candidates.into_iter().find(|path| path.exists())
}

#[cfg(feature = "word-convert")]
#[test]
fn word_convert_pdf_to_docx_returns_bytes_and_expected_text() {
    let processor = PdfProcessor::load(asset_path("example.pdf")).unwrap();
    let word = processor
        .to_word(PdfToWordOptions {
            preserve_page_breaks: true,
            empty_page_behavior: EmptyPageBehavior::Keep,
        })
        .unwrap();

    let bytes = word.into_bytes();
    assert!(!bytes.is_empty());
    assert!(String::from_utf8_lossy(&bytes).contains("word/document.xml"));
}

#[cfg(feature = "word-convert")]
#[test]
fn word_convert_docx_to_pdf_generates_loadable_pdf() {
    let font_path = find_system_font().expect("system font required for word-convert test");
    let docx = build_test_docx(&[
        ("Hello from DOCX", false, false, false, 24),
        ("Second paragraph", true, true, true, 28),
    ]);

    let pdf = PdfProcessor::from_word_bytes(
        &docx,
        WordToPdfOptions {
            default_font_path: Some(font_path),
            ..WordToPdfOptions::default()
        },
    )
    .unwrap();

    assert!(pdf.page_count() > 0);
    assert!(!pdf.into_bytes().unwrap().is_empty());
}

#[cfg(feature = "word-convert")]
#[test]
fn word_convert_round_trip_from_pdf_to_docx_back_to_pdf() {
    let font_path = find_system_font().expect("system font required for word-convert test");
    let source = PdfProcessor::load(asset_path("example.pdf")).unwrap();
    let word = source.to_word(Default::default()).unwrap();
    let recreated = PdfProcessor::from_word_bytes(
        word.as_bytes(),
        WordToPdfOptions {
            default_font_path: Some(font_path),
            ..WordToPdfOptions::default()
        },
    )
    .unwrap();

    assert!(recreated.page_count() > 0);
}

#[cfg(feature = "word-convert")]
#[test]
fn word_convert_rejects_legacy_doc_input() {
    let legacy_doc_magic = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    let error = PdfProcessor::from_word_bytes(&legacy_doc_magic, WordToPdfOptions::default())
        .unwrap_err();
    assert!(error.to_string().contains(".doc"));
    assert!(matches!(WordInputFormat::from_bytes(&legacy_doc_magic), Ok(WordInputFormat::Doc)));
}

#[cfg(feature = "word-convert")]
fn build_test_docx(paragraphs: &[(&str, bool, bool, bool, u32)]) -> Vec<u8> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#,
    );

    for (text, bold, italic, underline, size) in paragraphs {
        xml.push_str("<w:p><w:r><w:rPr>");
        if *bold {
            xml.push_str("<w:b/>");
        }
        if *italic {
            xml.push_str("<w:i/>");
        }
        if *underline {
            xml.push_str(r#"<w:u w:val="single"/>"#);
        }
        xml.push_str(&format!(r#"<w:sz w:val="{}"/>"#, size));
        xml.push_str("</w:rPr><w:t>");
        xml.push_str(text);
        xml.push_str("</w:t></w:r></w:p>");
    }

    xml.push_str(
        r#"<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr></w:body></w:document>"#,
    );

    let mut zip = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file("[Content_Types].xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
    )
    .unwrap();
    zip.start_file("_rels/.rels", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
    )
    .unwrap();
    zip.start_file("word/document.xml", options).unwrap();
    zip.write_all(xml.as_bytes()).unwrap();
    zip.start_file("word/_rels/document.xml.rels", options)
        .unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#,
    )
    .unwrap();

    zip.finish().unwrap().into_inner()
}
