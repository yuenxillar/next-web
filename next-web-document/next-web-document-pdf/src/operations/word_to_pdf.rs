use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use image::{DynamicImage, Rgba, RgbaImage};
use lopdf::{
    Document, Object, Stream, dictionary,
};
use lopdf::{
    content::{Content, Operation},
    ObjectId,
};
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use rusttype::{Font, Scale, point};
use zip::ZipArchive;

use crate::{PdfResult, error::pdf_error::PdfError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PdfPageSize {
    A4,
    Letter,
    Custom { width_pt: f32, height_pt: f32 },
}

impl Default for PdfPageSize {
    fn default() -> Self {
        Self::A4
    }
}

impl PdfPageSize {
    fn dimensions_pt(self) -> (f32, f32) {
        match self {
            Self::A4 => (595.0, 842.0),
            Self::Letter => (612.0, 792.0),
            Self::Custom {
                width_pt,
                height_pt,
            } => (width_pt.max(1.0), height_pt.max(1.0)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordInputFormat {
    Docx,
    Doc,
}

impl WordInputFormat {
    pub fn from_path(path: &Path) -> PdfResult<Self> {
        let ext = path
            .extension()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                PdfError::UnsupportedWordFormat(
                    "missing file extension; only .docx is supported".to_string(),
                )
            })?;

        match ext.to_ascii_lowercase().as_str() {
            "docx" => Ok(Self::Docx),
            "doc" => Ok(Self::Doc),
            other => Err(PdfError::UnsupportedWordFormat(format!(
                "unsupported Word extension .{other}; only .docx is supported"
            ))),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> PdfResult<Self> {
        if bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
            Ok(Self::Docx)
        } else if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
            Ok(Self::Doc)
        } else {
            Err(PdfError::WordParseFailed(
                "input bytes are not a recognizable .docx archive".to_string(),
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub struct WordToPdfOptions {
    pub page_size: PdfPageSize,
    pub margin_pt: f32,
    pub default_font_path: Option<PathBuf>,
    pub font_size_pt: f32,
    pub line_height: f32,
}

impl Default for WordToPdfOptions {
    fn default() -> Self {
        Self {
            page_size: PdfPageSize::A4,
            margin_pt: 48.0,
            default_font_path: None,
            font_size_pt: 12.0,
            line_height: 1.35,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WordToPdfOperation {
    source: WordSource,
    options: WordToPdfOptions,
}

#[derive(Debug, Clone)]
enum WordSource {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

impl WordToPdfOperation {
    pub fn from_path<P: AsRef<Path>>(path: P, options: WordToPdfOptions) -> Self {
        Self {
            source: WordSource::Path(path.as_ref().to_path_buf()),
            options,
        }
    }

    pub fn from_bytes(bytes: &[u8], options: WordToPdfOptions) -> Self {
        Self {
            source: WordSource::Bytes(bytes.to_vec()),
            options,
        }
    }

    pub fn convert(self) -> PdfResult<Document> {
        let format = match &self.source {
            WordSource::Path(path) => WordInputFormat::from_path(path)?,
            WordSource::Bytes(bytes) => WordInputFormat::from_bytes(bytes)?,
        };

        if matches!(format, WordInputFormat::Doc) {
            return Err(PdfError::UnsupportedWordFormat(
                "legacy .doc input is not supported; use .docx".to_string(),
            ));
        }

        let bytes = match self.source {
            WordSource::Path(path) => fs::read(path)?,
            WordSource::Bytes(bytes) => bytes,
        };

        let font_path = self
            .options
            .default_font_path
            .clone()
            .or_else(find_system_font)
            .ok_or_else(|| {
                PdfError::RuntimeDependencyMissing(
                    "no usable system font found; set WordToPdfOptions.default_font_path"
                        .to_string(),
                )
            })?;

        let font_data = fs::read(&font_path)?;
        let font = Font::try_from_vec(font_data).ok_or_else(|| {
            PdfError::WordConversionFailed(format!(
                "failed to parse font file: {}",
                font_path.display()
            ))
        })?;

        let parsed = parse_docx(&bytes, self.options.font_size_pt)?;
        render_docx_to_pdf(&parsed, &font, &self.options)
    }
}

#[derive(Debug, Clone)]
enum WordBlock {
    Paragraph(Vec<WordRun>),
    PageBreak,
}

#[derive(Debug, Clone)]
struct WordRun {
    text: String,
    style: RunStyle,
}

#[derive(Debug, Clone, Copy)]
struct RunStyle {
    bold: bool,
    italic: bool,
    underline: bool,
    font_size_pt: f32,
}

impl RunStyle {
    fn with_default_size(font_size_pt: f32) -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            font_size_pt,
        }
    }
}

#[derive(Debug, Clone)]
struct ParsedWordDocument {
    blocks: Vec<WordBlock>,
}

#[derive(Debug, Clone)]
struct StyledToken {
    text: String,
    style: RunStyle,
    font_size_px: f32,
}

#[derive(Debug, Clone, Default)]
struct LayoutLine {
    segments: Vec<StyledToken>,
    width_px: f32,
    max_font_px: f32,
}

impl LayoutLine {
    fn new() -> Self {
        Self {
            segments: Vec::new(),
            width_px: 0.0,
            max_font_px: 0.0,
        }
    }

    fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    fn push(&mut self, token: StyledToken, width_px: f32, font_size_px: f32) {
        self.width_px += width_px;
        self.max_font_px = self.max_font_px.max(font_size_px);
        self.segments.push(token);
    }
}

fn parse_docx(bytes: &[u8], default_font_size_pt: f32) -> PdfResult<ParsedWordDocument> {
    let cursor = Cursor::new(bytes.to_vec());
    let mut archive = ZipArchive::new(cursor)
        .map_err(|error| PdfError::WordParseFailed(error.to_string()))?;
    let mut file = archive
        .by_name("word/document.xml")
        .map_err(|error| PdfError::WordParseFailed(error.to_string()))?;
    let mut xml = String::new();
    file.read_to_string(&mut xml)
        .map_err(|error| PdfError::WordParseFailed(error.to_string()))?;

    parse_document_xml(&xml, default_font_size_pt)
}

fn parse_document_xml(xml: &str, default_font_size_pt: f32) -> PdfResult<ParsedWordDocument> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::new();
    let mut blocks = Vec::new();
    let mut current_paragraph: Option<Vec<WordRun>> = None;
    let mut current_run: Option<WordRun> = None;
    let mut current_style = RunStyle::with_default_size(default_font_size_pt);
    let mut in_text = false;
    let mut in_run_properties = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) => {
                handle_start_event(
                    &event,
                    reader.decoder(),
                    default_font_size_pt,
                    &mut blocks,
                    &mut current_paragraph,
                    &mut current_run,
                    &mut current_style,
                    &mut in_text,
                    &mut in_run_properties,
                )?;
            }
            Ok(Event::Empty(event)) => {
                handle_start_event(
                    &event,
                    reader.decoder(),
                    default_font_size_pt,
                    &mut blocks,
                    &mut current_paragraph,
                    &mut current_run,
                    &mut current_style,
                    &mut in_text,
                    &mut in_run_properties,
                )?;

                match local_name(&event).as_str() {
                    "r" => finalize_run(&mut current_paragraph, &mut current_run),
                    "rPr" => in_run_properties = false,
                    "t" => in_text = false,
                    _ => {}
                }
            }
            Ok(Event::Text(text)) => {
                if in_text {
                    let run = ensure_run(
                        &mut current_paragraph,
                        &mut current_run,
                        default_font_size_pt,
                        current_style,
                    );
                    run.text.push_str(
                        &text
                            .unescape()
                            .map_err(|error| PdfError::WordParseFailed(error.to_string()))?,
                    );
                }
            }
            Ok(Event::End(event)) => match local_name_end(&event).as_str() {
                "t" => in_text = false,
                "r" => finalize_run(&mut current_paragraph, &mut current_run),
                "rPr" => in_run_properties = false,
                "p" => finalize_paragraph(&mut blocks, &mut current_paragraph, &mut current_run),
                _ => {}
            },
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(PdfError::WordParseFailed(error.to_string()));
            }
        }

        buf.clear();
    }

    finalize_paragraph(&mut blocks, &mut current_paragraph, &mut current_run);

    if blocks.is_empty() {
        return Err(PdfError::WordElementUnsupported(
            "document contains no supported paragraphs or runs".to_string(),
        ));
    }

    Ok(ParsedWordDocument { blocks })
}

#[allow(clippy::too_many_arguments)]
fn handle_start_event(
    event: &BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
    default_font_size_pt: f32,
    blocks: &mut Vec<WordBlock>,
    current_paragraph: &mut Option<Vec<WordRun>>,
    current_run: &mut Option<WordRun>,
    current_style: &mut RunStyle,
    in_text: &mut bool,
    in_run_properties: &mut bool,
) -> PdfResult<()> {
    match local_name(event).as_str() {
        "p" => {
            finalize_paragraph(blocks, current_paragraph, current_run);
            *current_paragraph = Some(Vec::new());
        }
        "r" => {
            finalize_run(current_paragraph, current_run);
            if current_paragraph.is_none() {
                *current_paragraph = Some(Vec::new());
            }
            *current_style = RunStyle::with_default_size(default_font_size_pt);
            *current_run = Some(WordRun {
                text: String::new(),
                style: *current_style,
            });
        }
        "rPr" => {
            *in_run_properties = true;
        }
        "t" => {
            *in_text = true;
            ensure_run(
                current_paragraph,
                current_run,
                default_font_size_pt,
                *current_style,
            );
        }
        "br" => {
            let break_type = attribute_value(event, decoder, b"type");
            if matches!(break_type.as_deref(), Some("page")) {
                finalize_paragraph(blocks, current_paragraph, current_run);
                blocks.push(WordBlock::PageBreak);
            } else {
                let run = ensure_run(
                    current_paragraph,
                    current_run,
                    default_font_size_pt,
                    *current_style,
                );
                run.text.push('\n');
            }
        }
        "tab" => {
            let run = ensure_run(
                current_paragraph,
                current_run,
                default_font_size_pt,
                *current_style,
            );
            run.text.push('\t');
        }
        "b" if *in_run_properties => {
            current_style.bold = true;
            if let Some(run) = current_run.as_mut() {
                run.style.bold = true;
            }
        }
        "i" if *in_run_properties => {
            current_style.italic = true;
            if let Some(run) = current_run.as_mut() {
                run.style.italic = true;
            }
        }
        "u" if *in_run_properties => {
            let enabled = attribute_value(event, decoder, b"val")
                .map(|value| !value.eq_ignore_ascii_case("none"))
                .unwrap_or(true);
            current_style.underline = enabled;
            if let Some(run) = current_run.as_mut() {
                run.style.underline = enabled;
            }
        }
        "sz" if *in_run_properties => {
            if let Some(size) = attribute_value(event, decoder, b"val")
                .and_then(|value| value.parse::<f32>().ok())
            {
                current_style.font_size_pt = (size / 2.0).max(1.0);
                if let Some(run) = current_run.as_mut() {
                    run.style.font_size_pt = current_style.font_size_pt;
                }
            }
        }
        _ => {}
    }

    Ok(())
}

fn local_name(event: &BytesStart<'_>) -> String {
    std::str::from_utf8(event.local_name().as_ref())
        .unwrap_or_default()
        .to_string()
}

fn local_name_end(event: &quick_xml::events::BytesEnd<'_>) -> String {
    std::str::from_utf8(event.local_name().as_ref())
        .unwrap_or_default()
        .to_string()
}

fn attribute_value(
    event: &BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
    key_suffix: &[u8],
) -> Option<String> {
    event
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.as_ref().ends_with(key_suffix))
        .and_then(|attribute| attribute.decode_and_unescape_value(decoder).ok())
        .map(|value| value.into_owned())
}

fn ensure_run<'a>(
    current_paragraph: &'a mut Option<Vec<WordRun>>,
    current_run: &'a mut Option<WordRun>,
    default_font_size_pt: f32,
    current_style: RunStyle,
) -> &'a mut WordRun {
    if current_paragraph.is_none() {
        *current_paragraph = Some(Vec::new());
    }

    current_run.get_or_insert_with(|| WordRun {
        text: String::new(),
        style: if current_style.font_size_pt > 0.0 {
            current_style
        } else {
            RunStyle::with_default_size(default_font_size_pt)
        },
    })
}

fn finalize_run(current_paragraph: &mut Option<Vec<WordRun>>, current_run: &mut Option<WordRun>) {
    let Some(run) = current_run.take() else {
        return;
    };

    if current_paragraph.is_none() {
        *current_paragraph = Some(Vec::new());
    }

    if let Some(paragraph) = current_paragraph.as_mut() {
        if !run.text.is_empty() {
            paragraph.push(run);
        }
    }
}

fn finalize_paragraph(
    blocks: &mut Vec<WordBlock>,
    current_paragraph: &mut Option<Vec<WordRun>>,
    current_run: &mut Option<WordRun>,
) {
    finalize_run(current_paragraph, current_run);
    let Some(paragraph) = current_paragraph.take() else {
        return;
    };

    if paragraph.is_empty() {
        blocks.push(WordBlock::Paragraph(Vec::new()));
    } else {
        blocks.push(WordBlock::Paragraph(paragraph));
    }
}

fn render_docx_to_pdf(
    document: &ParsedWordDocument,
    font: &Font<'_>,
    options: &WordToPdfOptions,
) -> PdfResult<Document> {
    let page_images = layout_document_pages(document, font, options)?;
    build_pdf_from_images(&page_images, options.page_size)
}

fn layout_document_pages(
    document: &ParsedWordDocument,
    font: &Font<'_>,
    options: &WordToPdfOptions,
) -> PdfResult<Vec<RgbaImage>> {
    let scale_factor = 2.0f32;
    let (page_width_pt, page_height_pt) = options.page_size.dimensions_pt();
    let page_width_px = (page_width_pt * scale_factor).round().max(1.0) as u32;
    let page_height_px = (page_height_pt * scale_factor).round().max(1.0) as u32;
    let margin_px = (options.margin_pt * scale_factor).round().max(0.0);
    let max_width_px = (page_width_px as f32 - margin_px * 2.0).max(1.0);
    let bottom_limit_px = page_height_px as f32 - margin_px;

    let mut pages = vec![new_page_image(page_width_px, page_height_px)];
    let mut page_index = 0usize;
    let mut cursor_y = margin_px;

    for block in &document.blocks {
        match block {
            WordBlock::PageBreak => {
                page_index += 1;
                pages.push(new_page_image(page_width_px, page_height_px));
                cursor_y = margin_px;
            }
            WordBlock::Paragraph(runs) => {
                let lines = layout_paragraph_lines(runs, font, max_width_px, scale_factor, options);

                if lines.is_empty() {
                    let blank_height = (options.font_size_pt * options.line_height * scale_factor)
                        .max(1.0);
                    if cursor_y + blank_height > bottom_limit_px {
                        page_index += 1;
                        pages.push(new_page_image(page_width_px, page_height_px));
                        cursor_y = margin_px;
                    }
                    cursor_y += blank_height;
                    continue;
                }

                for line in lines {
                    let line_height_px = (line.max_font_px * options.line_height).max(1.0);
                    if cursor_y + line_height_px > bottom_limit_px {
                        page_index += 1;
                        pages.push(new_page_image(page_width_px, page_height_px));
                        cursor_y = margin_px;
                    }

                    let page = pages.get_mut(page_index).ok_or_else(|| {
                        PdfError::WordConversionFailed(
                            "failed to allocate target page image".to_string(),
                        )
                    })?;

                    draw_layout_line(page, font, margin_px, cursor_y, &line);
                    cursor_y += line_height_px;
                }

                cursor_y += (options.font_size_pt * scale_factor * 0.35).max(2.0);
            }
        }
    }

    Ok(pages)
}

fn layout_paragraph_lines(
    runs: &[WordRun],
    font: &Font<'_>,
    max_width_px: f32,
    scale_factor: f32,
    options: &WordToPdfOptions,
) -> Vec<LayoutLine> {
    let mut lines = Vec::new();
    let mut current = LayoutLine::new();

    for run in runs {
        for token_text in tokenize_run_text(&run.text) {
            if token_text == "\n" {
                lines.push(std::mem::take(&mut current));
                continue;
            }

            if token_text.trim().is_empty() && current.is_empty() {
                continue;
            }

            let font_size_px = (run.style.font_size_pt.max(options.font_size_pt) * scale_factor)
                .max(1.0);
            let token_width = measure_text_width(font, &token_text, font_size_px);

            if !current.is_empty() && !token_text.trim().is_empty() && current.width_px + token_width > max_width_px {
                lines.push(std::mem::take(&mut current));
            }

            if measure_text_width(font, &token_text, font_size_px) > max_width_px {
                for part in split_token_to_fit(font, &token_text, font_size_px, max_width_px) {
                    if !current.is_empty()
                        && !part.trim().is_empty()
                        && current.width_px + measure_text_width(font, &part, font_size_px)
                            > max_width_px
                    {
                        lines.push(std::mem::take(&mut current));
                    }
                    if current.is_empty() && part.trim().is_empty() {
                        continue;
                    }
                    current.push(
                        StyledToken {
                            text: part.clone(),
                            style: run.style,
                            font_size_px,
                        },
                        measure_text_width(font, &part, font_size_px),
                        font_size_px,
                    );
                }
                continue;
            }

            if current.is_empty() && token_text.trim().is_empty() {
                continue;
            }

            current.push(
                StyledToken {
                    text: token_text.clone(),
                    style: run.style,
                    font_size_px,
                },
                token_width,
                font_size_px,
            );
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn tokenize_run_text(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_kind: Option<bool> = None;

    for ch in text.chars() {
        if ch == '\n' {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            current_kind = None;
            tokens.push("\n".to_string());
            continue;
        }

        let is_whitespace = ch.is_whitespace();
        match current_kind {
            Some(kind) if kind == is_whitespace => current.push(ch),
            Some(_) => {
                tokens.push(std::mem::take(&mut current));
                current.push(ch);
                current_kind = Some(is_whitespace);
            }
            None => {
                current.push(ch);
                current_kind = Some(is_whitespace);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn split_token_to_fit(
    font: &Font<'_>,
    token: &str,
    font_size_px: f32,
    max_width_px: f32,
) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();

    for ch in token.chars() {
        current.push(ch);
        if measure_text_width(font, &current, font_size_px) > max_width_px && current.chars().count() > 1 {
            let last = current.pop().unwrap_or_default();
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
            current.push(last);
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

fn measure_text_width(font: &Font<'_>, text: &str, font_size_px: f32) -> f32 {
    let scale = Scale::uniform(font_size_px);
    let mut width = 0.0f32;

    for ch in text.chars() {
        if ch == '\t' {
            width += measure_text_width(font, "    ", font_size_px);
            continue;
        }

        let glyph = font.glyph(ch).scaled(scale);
        width += glyph.h_metrics().advance_width;
    }

    width
}

fn draw_layout_line(
    image: &mut RgbaImage,
    font: &Font<'_>,
    start_x: f32,
    top_y: f32,
    line: &LayoutLine,
) {
    let mut cursor_x = start_x;

    for segment in &line.segments {
        let font_size_px = segment.font_size_px.max(1.0);
        let scale = Scale::uniform(font_size_px);
        let v_metrics = font.v_metrics(scale);
        let baseline = top_y + v_metrics.ascent;

        draw_text_segment(
            image,
            font,
            &segment.text.replace('\t', "    "),
            cursor_x,
            baseline,
            segment.style,
            font_size_px,
        );

        let segment_width = measure_text_width(font, &segment.text.replace('\t', "    "), font_size_px);
        if segment.style.underline && !segment.text.trim().is_empty() {
            let underline_y = (baseline + font_size_px * 0.08).round() as i32;
            draw_horizontal_line(
                image,
                cursor_x.round() as i32,
                underline_y,
                (cursor_x + segment_width).round() as i32,
            );
        }

        cursor_x += segment_width;
    }
}

fn draw_text_segment(
    image: &mut RgbaImage,
    font: &Font<'_>,
    text: &str,
    x: f32,
    baseline: f32,
    style: RunStyle,
    font_size_px: f32,
) {
    let scale = Scale::uniform(font_size_px);
    let glyphs: Vec<_> = font.layout(text, scale, point(x, baseline)).collect();

    for glyph in glyphs {
        if let Some(bounds) = glyph.pixel_bounding_box() {
            glyph.draw(|gx, gy, coverage| {
                let mut target_x = gx as i32 + bounds.min.x;
                let target_y = gy as i32 + bounds.min.y;

                if style.italic {
                    let slant = ((bounds.max.y - target_y) as f32 * 0.18).round() as i32;
                    target_x += slant;
                }

                blend_pixel(image, target_x, target_y, coverage);

                if style.bold {
                    blend_pixel(image, target_x + 1, target_y, coverage);
                }
            });
        }
    }
}

fn blend_pixel(image: &mut RgbaImage, x: i32, y: i32, coverage: f32) {
    if x < 0 || y < 0 {
        return;
    }

    let x = x as u32;
    let y = y as u32;
    if x >= image.width() || y >= image.height() {
        return;
    }

    let alpha = (coverage.clamp(0.0, 1.0) * 255.0).round() as u8;
    let pixel = image.get_pixel_mut(x, y);
    let shade = 255u8.saturating_sub(alpha);
    pixel.0[0] = pixel.0[0].min(shade);
    pixel.0[1] = pixel.0[1].min(shade);
    pixel.0[2] = pixel.0[2].min(shade);
    pixel.0[3] = 255;
}

fn draw_horizontal_line(image: &mut RgbaImage, x1: i32, y: i32, x2: i32) {
    for x in x1.min(x2)..=x1.max(x2) {
        blend_pixel(image, x, y, 1.0);
    }
}

fn new_page_image(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_pixel(width, height, Rgba([255, 255, 255, 255]))
}

fn build_pdf_from_images(images: &[RgbaImage], page_size: PdfPageSize) -> PdfResult<Document> {
    let (width_pt, height_pt) = page_size.dimensions_pt();
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::with_capacity(images.len());

    for (index, image) in images.iter().enumerate() {
        let resource_name = format!("Im{}", index + 1);
        let image_id = add_page_image_object(
            &mut doc,
            &DynamicImage::ImageRgba8(image.clone()),
        )?;
        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![
                        width_pt.into(),
                        0.into(),
                        0.into(),
                        height_pt.into(),
                        0.into(),
                        0.into(),
                    ],
                ),
                Operation::new("Do", vec![Object::Name(resource_name.as_bytes().to_vec())]),
                Operation::new("Q", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode()?));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => dictionary! {
                "XObject" => dictionary! {
                    resource_name.as_str() => image_id,
                }
            },
            "MediaBox" => vec![0.into(), 0.into(), width_pt.into(), height_pt.into()],
        });
        kids.push(page_id.into());
    }

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => images.len() as i64,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.compress();
    Ok(doc)
}

fn add_page_image_object(doc: &mut Document, image: &DynamicImage) -> PdfResult<ObjectId> {
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut rgb = Vec::with_capacity((width as usize) * (height as usize) * 3);

    for pixel in rgba.pixels() {
        rgb.extend_from_slice(&pixel.0[..3]);
    }

    let mut image_dict = lopdf::Dictionary::new();
    image_dict.set("Type", Object::Name(b"XObject".to_vec()));
    image_dict.set("Subtype", Object::Name(b"Image".to_vec()));
    image_dict.set("Width", width);
    image_dict.set("Height", height);
    image_dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
    image_dict.set("BitsPerComponent", 8);

    Ok(doc.add_object(Stream::new(image_dict, rgb)))
}

fn find_system_font() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from(r"C:\Windows\Fonts\msyh.ttc"),
        PathBuf::from(r"C:\Windows\Fonts\msyh.ttf"),
        PathBuf::from(r"C:\Windows\Fonts\simhei.ttf"),
        PathBuf::from(r"C:\Windows\Fonts\arial.ttf"),
        PathBuf::from(r"C:\Windows\Fonts\calibri.ttf"),
        PathBuf::from("/System/Library/Fonts/Supplemental/Arial Unicode.ttf"),
        PathBuf::from("/System/Library/Fonts/Supplemental/Arial.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
        PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"),
    ];

    candidates.into_iter().find(|path| path.exists())
}
