use std::{
    io::{Cursor, Write},
    path::Path,
};

use lopdf::Document;
use zip::{
    CompressionMethod, ZipWriter,
    write::SimpleFileOptions,
};

use crate::{
    PdfResult,
    error::pdf_error::PdfError,
    operation::PdfQuery,
    operations::extract_text::PdfExtractTextOperation,
    util::save_bytes,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmptyPageBehavior {
    Keep,
    Skip,
    Placeholder(String),
}

impl Default for EmptyPageBehavior {
    fn default() -> Self {
        Self::Keep
    }
}

#[derive(Debug, Clone)]
pub struct PdfToWordOptions {
    pub preserve_page_breaks: bool,
    pub empty_page_behavior: EmptyPageBehavior,
}

impl Default for PdfToWordOptions {
    fn default() -> Self {
        Self {
            preserve_page_breaks: true,
            empty_page_behavior: EmptyPageBehavior::Keep,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WordDocument {
    bytes: Vec<u8>,
}

impl WordDocument {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> PdfResult<()> {
        let path = path.as_ref();
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("doc"))
        {
            return Err(PdfError::UnsupportedWordFormat(
                "legacy .doc output is not supported; use .docx".to_string(),
            ));
        }

        save_bytes(path, &self.bytes)
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone)]
pub struct PdfToWordOperation {
    options: PdfToWordOptions,
}

impl PdfToWordOperation {
    pub fn new(options: PdfToWordOptions) -> Self {
        Self { options }
    }
}

impl PdfQuery<WordDocument> for PdfToWordOperation {
    fn query(&self, doc: &Document) -> PdfResult<WordDocument> {
        let mut pages = Vec::new();

        for page_number in doc.get_pages().keys().copied() {
            let text = PdfExtractTextOperation::from_pages([page_number]).query(doc)?;
            if let Some(paragraphs) = paragraphs_for_page(&text, &self.options.empty_page_behavior) {
                pages.push(paragraphs);
            }
        }

        let bytes = build_docx_bytes(&pages, self.options.preserve_page_breaks)?;
        Ok(WordDocument::new(bytes))
    }
}

fn paragraphs_for_page(text: &str, behavior: &EmptyPageBehavior) -> Option<Vec<String>> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");

    if normalized.trim().is_empty() {
        return match behavior {
            EmptyPageBehavior::Keep => Some(vec![String::new()]),
            EmptyPageBehavior::Skip => None,
            EmptyPageBehavior::Placeholder(text) => Some(vec![text.clone()]),
        };
    }

    let mut paragraphs = Vec::new();
    let mut current = Vec::new();

    for line in normalized.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join("\n"));
                current.clear();
            } else {
                paragraphs.push(String::new());
            }
        } else {
            current.push(line.to_string());
        }
    }

    if !current.is_empty() {
        paragraphs.push(current.join("\n"));
    }

    if paragraphs.is_empty() {
        Some(vec![String::new()])
    } else {
        Some(paragraphs)
    }
}

fn build_docx_bytes(pages: &[Vec<String>], preserve_page_breaks: bool) -> PdfResult<Vec<u8>> {
    let document_xml = build_document_xml(pages, preserve_page_breaks);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);

    write_zip_file(&mut zip, "[Content_Types].xml", CONTENT_TYPES_XML, options)?;
    write_zip_file(&mut zip, "_rels/.rels", ROOT_RELS_XML, options)?;
    write_zip_file(&mut zip, "word/document.xml", &document_xml, options)?;
    write_zip_file(
        &mut zip,
        "word/_rels/document.xml.rels",
        DOCUMENT_RELS_XML,
        options,
    )?;

    zip.finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|error| PdfError::WordConversionFailed(error.to_string()))
}

fn write_zip_file(
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    name: &str,
    content: &str,
    options: SimpleFileOptions,
) -> PdfResult<()> {
    zip.start_file(name, options)
        .map_err(|error| PdfError::WordConversionFailed(error.to_string()))?;
    zip.write_all(content.as_bytes())
        .map_err(PdfError::from)
}

fn build_document_xml(pages: &[Vec<String>], preserve_page_breaks: bool) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    );
    xml.push_str(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#,
    );

    for (page_index, paragraphs) in pages.iter().enumerate() {
        if paragraphs.is_empty() {
            xml.push_str("<w:p/>");
        } else {
            for paragraph in paragraphs {
                append_paragraph_xml(&mut xml, paragraph);
            }
        }

        if preserve_page_breaks && page_index + 1 < pages.len() {
            xml.push_str(r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#);
        }
    }

    xml.push_str(
        r#"<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr>"#,
    );
    xml.push_str("</w:body></w:document>");
    xml
}

fn append_paragraph_xml(xml: &mut String, paragraph: &str) {
    if paragraph.is_empty() {
        xml.push_str("<w:p/>");
        return;
    }

    xml.push_str("<w:p>");

    let normalized = paragraph.replace("\r\n", "\n").replace('\r', "\n");
    let mut parts = normalized.split('\n').peekable();
    while let Some(part) = parts.next() {
        xml.push_str(r#"<w:r><w:t xml:space="preserve">"#);
        xml.push_str(&escape_xml_text(part));
        xml.push_str("</w:t></w:r>");

        if parts.peek().is_some() {
            xml.push_str(r#"<w:r><w:br/></w:r>"#);
        }
    }

    xml.push_str("</w:p>");
}

fn escape_xml_text(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

const CONTENT_TYPES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>
"#;

const ROOT_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>
"#;

const DOCUMENT_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>
"#;
