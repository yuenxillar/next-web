use std::{
    fs,
    path::{Path, PathBuf},
};

use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use rusttype::{Font, Scale, point};

use crate::{PdfResult, error::pdf_error::PdfError, operation::PdfOperation};

/// Watermark operation configuration.
pub struct PdfWatermarkOperation {
    /// Watermark payload.
    pub content: WatermarkType,
    /// Placement strategy.
    pub layout: WatermarkLayout,
    /// Rotation angle in degrees.
    pub angle: f64,
    /// Global opacity in the range [0.0, 1.0].
    pub opacity: f32,
    /// Legacy field kept for backwards compatibility. Positioning is controlled by `layout`.
    pub offset: (f64, f64),
}

impl PdfOperation<()> for PdfWatermarkOperation {
    fn execute(self, doc: &mut Document) -> PdfResult<()> {
        let gs_id = add_transparency_group(doc, self.opacity)?;
        let prepared = PreparedWatermark::prepare(doc, &self.content)?;
        let page_ids: Vec<ObjectId> = doc.get_pages().values().copied().collect();

        for page_id in page_ids {
            let page_size = resolve_page_size(doc, page_id)?;
            let page_resources = resolve_page_resources(doc, page_id)?;

            {
                let page = doc
                    .get_object_mut(page_id)
                    .and_then(|obj| obj.as_dict_mut())
                    .map_err(|error| {
                        PdfError::AnalysisError(format!(
                            "failed to get page dictionary {page_id:?}: {error}"
                        ))
                    })?;

                page.set("Resources", Object::Dictionary(page_resources));
            }

            let page = doc
                .get_object_mut(page_id)
                .and_then(|obj| obj.as_dict_mut())
                .map_err(|error| {
                    PdfError::AnalysisError(format!(
                        "failed to re-open page dictionary {page_id:?}: {error}"
                    ))
                })?;

            let resources = page
                .get_mut(b"Resources")
                .and_then(|obj| obj.as_dict_mut())
                .map_err(|error| {
                    PdfError::AnalysisError(format!(
                        "failed to get page resources {page_id:?}: {error}"
                    ))
                })?;

            register_resource(resources, "ExtGState", "WsGS", Object::Reference(gs_id));
            prepared.register_resources(resources);

            let coordinates = self.layout.resolve_points(
                page_size.width,
                page_size.height,
                prepared.width,
                prepared.height,
            );

            let angle_rad = self.angle.to_radians();
            let c = angle_rad.cos();
            let s = angle_rad.sin();
            let mut operations = Vec::new();

            for (cx, cy) in coordinates {
                operations.push(Operation::new("q", vec![]));
                operations.push(Operation::new("gs", vec![Object::Name(b"WsGS".to_vec())]));
                operations.push(Operation::new(
                    "cm",
                    vec![
                        c.into(),
                        s.into(),
                        (-s).into(),
                        c.into(),
                        cx.into(),
                        cy.into(),
                    ],
                ));

                prepared.append_operations(&mut operations);
                operations.push(Operation::new("Q", vec![]));
            }

            append_content_stream(doc, page_id, operations)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct PreparedWatermark {
    width: f64,
    height: f64,
    kind: PreparedWatermarkKind,
}

#[derive(Debug, Clone)]
enum PreparedWatermarkKind {
    StandardText {
        content: String,
        font_name: String,
        font_size: f32,
        color_rgb: (f32, f32, f32),
    },
    Image {
        resource_name: String,
        object_id: ObjectId,
    },
}

impl PreparedWatermark {
    fn prepare(doc: &mut Document, content: &WatermarkType) -> PdfResult<Self> {
        match content {
            WatermarkType::Text {
                content,
                font_name,
                font_path,
                font_size,
                color_rgb,
            } => {
                if let Some(font_path) = font_path {
                    let rendered = render_text_to_image(font_path, content, *font_size, *color_rgb)?;
                    let (width, height) = rendered.dimensions();
                    let object_id = add_dynamic_image_object(doc, &rendered)?;
                    Ok(Self {
                        width: f64::from(width),
                        height: f64::from(height),
                        kind: PreparedWatermarkKind::Image {
                            resource_name: "WsTextImage".to_string(),
                            object_id,
                        },
                    })
                } else {
                    let width = estimate_text_width(content, *font_size);
                    let height = f64::from(*font_size) * 1.2;
                    Ok(Self {
                        width,
                        height,
                        kind: PreparedWatermarkKind::StandardText {
                            content: content.clone(),
                            font_name: font_name
                                .clone()
                                .unwrap_or_else(|| "Helvetica".to_string()),
                            font_size: *font_size,
                            color_rgb: *color_rgb,
                        },
                    })
                }
            }
            WatermarkType::Image {
                path,
                width,
                height,
            } => Ok(Self {
                width: f64::from(*width),
                height: f64::from(*height),
                kind: PreparedWatermarkKind::Image {
                    resource_name: "WsImage".to_string(),
                    object_id: add_image_object(doc, path)?,
                },
            }),
        }
    }

    fn register_resources(&self, resources: &mut Dictionary) {
        match &self.kind {
            PreparedWatermarkKind::StandardText { font_name, .. } => {
                let font_dict = Dictionary::from_iter(vec![
                    ("Type", "Font".into()),
                    ("Subtype", "Type1".into()),
                    ("BaseFont", Object::Name(font_name.as_bytes().to_vec())),
                ]);
                register_resource(resources, "Font", "WsFont", Object::Dictionary(font_dict));
            }
            PreparedWatermarkKind::Image {
                resource_name,
                object_id,
            } => {
                register_resource(
                    resources,
                    "XObject",
                    resource_name,
                    Object::Reference(*object_id),
                );
            }
        }
    }

    fn append_operations(&self, operations: &mut Vec<Operation>) {
        match &self.kind {
            PreparedWatermarkKind::StandardText {
                content,
                font_size,
                color_rgb,
                ..
            } => {
                let tx = -(self.width / 2.0);
                let ty = -(self.height / 2.8);

                operations.push(Operation::new("BT", vec![]));
                operations.push(Operation::new(
                    "rg",
                    vec![color_rgb.0.into(), color_rgb.1.into(), color_rgb.2.into()],
                ));
                operations.push(Operation::new(
                    "Tf",
                    vec![Object::Name(b"WsFont".to_vec()), (*font_size).into()],
                ));
                operations.push(Operation::new("Td", vec![tx.into(), ty.into()]));
                operations.push(Operation::new(
                    "Tj",
                    vec![Object::string_literal(content.as_bytes().to_vec())],
                ));
                operations.push(Operation::new("ET", vec![]));
            }
            PreparedWatermarkKind::Image { resource_name, .. } => {
                operations.push(Operation::new(
                    "cm",
                    vec![
                        self.width.into(),
                        0.into(),
                        0.into(),
                        self.height.into(),
                        (-self.width / 2.0).into(),
                        (-self.height / 2.0).into(),
                    ],
                ));
                operations.push(Operation::new(
                    "Do",
                    vec![Object::Name(resource_name.as_bytes().to_vec())],
                ));
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PageSize {
    width: f64,
    height: f64,
}

fn resolve_page_size(doc: &Document, page_id: ObjectId) -> PdfResult<PageSize> {
    let page = doc
        .get_object(page_id)
        .and_then(|obj| obj.as_dict())
        .map_err(|error| {
            PdfError::AnalysisError(format!("failed to get page dictionary {page_id:?}: {error}"))
        })?;

    let media_box = resolve_inherited_object(doc, page, b"MediaBox")?
        .ok_or_else(|| PdfError::AnalysisError("missing MediaBox".to_string()))?;
    let media_box = media_box.as_array().map_err(|error| {
        PdfError::AnalysisError(format!("MediaBox is not an array: {error}"))
    })?;

    if media_box.len() < 4 {
        return Err(PdfError::AnalysisError(
            "MediaBox must contain four entries".to_string(),
        ));
    }

    Ok(PageSize {
        width: media_box[2].as_f32().unwrap_or(595.0) as f64,
        height: media_box[3].as_f32().unwrap_or(842.0) as f64,
    })
}

fn resolve_page_resources(doc: &Document, page_id: ObjectId) -> PdfResult<Dictionary> {
    let page = doc
        .get_object(page_id)
        .and_then(|obj| obj.as_dict())
        .map_err(|error| {
            PdfError::AnalysisError(format!("failed to get page dictionary {page_id:?}: {error}"))
        })?;

    let resources = resolve_inherited_object(doc, page, b"Resources")?;
    match resources {
        Some(Object::Dictionary(dict)) => Ok(dict),
        Some(other) => other.as_dict().cloned().map_err(|error| {
            PdfError::AnalysisError(format!("Resources is not a dictionary: {error}"))
        }),
        None => Ok(Dictionary::new()),
    }
}

fn resolve_inherited_object(
    doc: &Document,
    start: &Dictionary,
    key: &[u8],
) -> PdfResult<Option<Object>> {
    let mut current = start.clone();

    for _ in 0..50 {
        if let Ok(object) = current.get(key) {
            return dereference_object(doc, object).map(Some);
        }

        let parent_id = match current.get(b"Parent").and_then(|obj| obj.as_reference()) {
            Ok(parent_id) => parent_id,
            Err(_) => break,
        };

        current = doc
            .get_object(parent_id)
            .and_then(|obj| obj.as_dict())
            .map_err(|error| {
                PdfError::AnalysisError(format!(
                    "failed to read page parent node {parent_id:?}: {error}"
                ))
            })?
            .clone();
    }

    Ok(None)
}

fn dereference_object(doc: &Document, object: &Object) -> PdfResult<Object> {
    match object {
        Object::Reference(object_id) => doc.get_object(*object_id).map(|obj| obj.clone()).map_err(Into::into),
        other => Ok(other.clone()),
    }
}

fn append_content_stream(
    doc: &mut Document,
    page_id: ObjectId,
    operations: Vec<Operation>,
) -> PdfResult<()> {
    let stream_id = doc.add_object(Stream::new(
        Dictionary::new(),
        Content { operations }.encode()?,
    ));

    match doc
        .get_object_mut(page_id)
        .and_then(|obj| obj.as_dict_mut())?
        .get_mut(b"Contents")
    {
        Ok(Object::Reference(id)) => {
            *doc.get_object_mut(page_id)
                .and_then(|obj| obj.as_dict_mut())?
                .get_mut(b"Contents")? =
                Object::Array(vec![Object::Reference(*id), Object::Reference(stream_id)]);
        }
        Ok(Object::Array(items)) => items.push(Object::Reference(stream_id)),
        Err(_) => {
            doc.get_object_mut(page_id)
                .and_then(|obj| obj.as_dict_mut())?
                .set("Contents", Object::Reference(stream_id));
        }
        _ => {}
    }

    Ok(())
}

fn add_transparency_group(doc: &mut Document, alpha: f32) -> PdfResult<ObjectId> {
    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name(b"ExtGState".to_vec()));
    dict.set("ca", Object::Real(alpha));
    dict.set("CA", Object::Real(alpha));
    Ok(doc.add_object(Object::Dictionary(dict)))
}

fn add_image_object(doc: &mut Document, path: &Path) -> PdfResult<ObjectId> {
    let image = image::open(path)
        .map_err(|error| PdfError::Io(std::io::Error::other(error.to_string())))?;
    add_dynamic_image_object(doc, &image)
}

fn add_dynamic_image_object(doc: &mut Document, image: &DynamicImage) -> PdfResult<ObjectId> {
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut rgb = Vec::with_capacity((width as usize) * (height as usize) * 3);
    let mut alpha = Vec::with_capacity((width as usize) * (height as usize));
    let mut has_alpha = false;

    for pixel in rgba.pixels() {
        rgb.extend_from_slice(&pixel.0[..3]);
        alpha.push(pixel[3]);
        has_alpha |= pixel[3] < 255;
    }

    let smask_id = if has_alpha {
        let mut mask_dict = Dictionary::new();
        mask_dict.set("Type", Object::Name(b"XObject".to_vec()));
        mask_dict.set("Subtype", Object::Name(b"Image".to_vec()));
        mask_dict.set("Width", width);
        mask_dict.set("Height", height);
        mask_dict.set("ColorSpace", Object::Name(b"DeviceGray".to_vec()));
        mask_dict.set("BitsPerComponent", 8);
        Some(doc.add_object(Stream::new(mask_dict, alpha)))
    } else {
        None
    };

    let mut image_dict = Dictionary::new();
    image_dict.set("Type", Object::Name(b"XObject".to_vec()));
    image_dict.set("Subtype", Object::Name(b"Image".to_vec()));
    image_dict.set("Width", width);
    image_dict.set("Height", height);
    image_dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
    image_dict.set("BitsPerComponent", 8);

    if let Some(smask_id) = smask_id {
        image_dict.set("SMask", Object::Reference(smask_id));
    }

    Ok(doc.add_object(Stream::new(image_dict, rgb)))
}

fn register_resource(resources: &mut Dictionary, kind: &str, name: &str, value: Object) {
    if let Ok(sub_dict) = resources
        .get_mut(kind.as_bytes())
        .and_then(|obj| obj.as_dict_mut())
    {
        sub_dict.set(name, value);
    } else {
        let mut sub_dict = Dictionary::new();
        sub_dict.set(name, value);
        resources.set(kind, sub_dict);
    }
}

fn estimate_text_width(content: &str, font_size: f32) -> f64 {
    let average = if content.is_ascii() { 0.52 } else { 0.9 };
    content.chars().count() as f64 * f64::from(font_size) * average
}

fn render_text_to_image(
    font_path: &Path,
    content: &str,
    font_size: f32,
    color_rgb: (f32, f32, f32),
) -> PdfResult<DynamicImage> {
    let font_data = fs::read(font_path)?;
    let font = Font::try_from_vec(font_data).ok_or_else(|| {
        PdfError::AnalysisError(format!(
            "failed to parse font file: {}",
            font_path.display()
        ))
    })?;

    let scale = Scale::uniform(font_size.max(1.0));
    let v_metrics = font.v_metrics(scale);
    let glyphs: Vec<_> = font
        .layout(content, scale, point(0.0, v_metrics.ascent))
        .collect();

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for glyph in &glyphs {
        if let Some(bounds) = glyph.pixel_bounding_box() {
            min_x = min_x.min(bounds.min.x);
            min_y = min_y.min(bounds.min.y);
            max_x = max_x.max(bounds.max.x);
            max_y = max_y.max(bounds.max.y);
        }
    }

    if min_x == i32::MAX || min_y == i32::MAX {
        return Ok(DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            1,
            1,
            Rgba([0, 0, 0, 0]),
        )));
    }

    let padding = 6i32;
    let width = (max_x - min_x + padding * 2).max(1) as u32;
    let height = (max_y - min_y + padding * 2).max(1) as u32;
    let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));

    let red = (color_rgb.0.clamp(0.0, 1.0) * 255.0).round() as u8;
    let green = (color_rgb.1.clamp(0.0, 1.0) * 255.0).round() as u8;
    let blue = (color_rgb.2.clamp(0.0, 1.0) * 255.0).round() as u8;

    for glyph in glyphs {
        if let Some(bounds) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, coverage| {
                let target_x = x as i32 + bounds.min.x - min_x + padding;
                let target_y = y as i32 + bounds.min.y - min_y + padding;

                if target_x < 0 || target_y < 0 {
                    return;
                }

                let target_x = target_x as u32;
                let target_y = target_y as u32;

                if target_x >= width || target_y >= height {
                    return;
                }

                let alpha = (coverage * 255.0).round() as u8;
                let pixel = image.get_pixel_mut(target_x, target_y);
                if alpha > pixel[3] {
                    *pixel = Rgba([red, green, blue, alpha]);
                }
            });
        }
    }

    Ok(DynamicImage::ImageRgba8(image))
}

/// Watermark payload.
pub enum WatermarkType {
    /// Text watermark.
    Text {
        content: String,
        /// Standard PDF base font name, for example `Helvetica`.
        font_name: Option<String>,
        /// Optional font file path. When provided, the text is rendered to a transparent image,
        /// allowing arbitrary TTF/OTF fonts and non-Latin text.
        font_path: Option<PathBuf>,
        font_size: f32,
        /// RGB color in the range [0.0, 1.0].
        color_rgb: (f32, f32, f32),
    },
    /// Image watermark.
    Image {
        path: PathBuf,
        /// Display width on the PDF page in points.
        width: f32,
        /// Display height on the PDF page in points.
        height: f32,
    },
}

/// Watermark anchor positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatermarkAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Watermark layout mode.
#[derive(Debug, Clone, PartialEq)]
pub enum WatermarkLayout {
    /// Single watermark centered on the page with an offset.
    Single { offset_x: f64, offset_y: f64 },
    /// Absolute page position in PDF points, using the PDF coordinate system.
    Absolute { x: f64, y: f64 },
    /// Anchor-based positioning with an offset from the anchor.
    Anchored {
        anchor: WatermarkAnchor,
        offset_x: f64,
        offset_y: f64,
    },
    /// Repeating tiled watermark.
    Tile {
        gap_x: f64,
        gap_y: f64,
        stagger: bool,
    },
}

impl WatermarkLayout {
    pub fn centered() -> Self {
        Self::Single {
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }

    pub fn offset(offset_x: f64, offset_y: f64) -> Self {
        Self::Single { offset_x, offset_y }
    }

    pub fn at(x: f64, y: f64) -> Self {
        Self::Absolute { x, y }
    }

    pub fn anchored(anchor: WatermarkAnchor, offset_x: f64, offset_y: f64) -> Self {
        Self::Anchored {
            anchor,
            offset_x,
            offset_y,
        }
    }

    pub fn tiled(gap_x: f64, gap_y: f64) -> Self {
        Self::Tile {
            gap_x,
            gap_y,
            stagger: false,
        }
    }

    fn resolve_points(
        &self,
        page_width: f64,
        page_height: f64,
        item_width: f64,
        item_height: f64,
    ) -> Vec<(f64, f64)> {
        match *self {
            WatermarkLayout::Single { offset_x, offset_y } => {
                vec![(page_width / 2.0 + offset_x, page_height / 2.0 + offset_y)]
            }
            WatermarkLayout::Absolute { x, y } => vec![(x, y)],
            WatermarkLayout::Anchored {
                anchor,
                offset_x,
                offset_y,
            } => {
                let half_w = item_width / 2.0;
                let half_h = item_height / 2.0;
                let point = match anchor {
                    WatermarkAnchor::TopLeft => (half_w, page_height - half_h),
                    WatermarkAnchor::TopCenter => (page_width / 2.0, page_height - half_h),
                    WatermarkAnchor::TopRight => (page_width - half_w, page_height - half_h),
                    WatermarkAnchor::CenterLeft => (half_w, page_height / 2.0),
                    WatermarkAnchor::Center => (page_width / 2.0, page_height / 2.0),
                    WatermarkAnchor::CenterRight => (page_width - half_w, page_height / 2.0),
                    WatermarkAnchor::BottomLeft => (half_w, half_h),
                    WatermarkAnchor::BottomCenter => (page_width / 2.0, half_h),
                    WatermarkAnchor::BottomRight => (page_width - half_w, half_h),
                };

                vec![(point.0 + offset_x, point.1 + offset_y)]
            }
            WatermarkLayout::Tile {
                gap_x,
                gap_y,
                stagger,
            } => {
                let mut points = Vec::new();
                let margin_x = gap_x / 2.0;
                let margin_y = gap_y / 2.0;

                let mut row_index = 0usize;
                let mut y = margin_y;
                while y < page_height {
                    let mut x = margin_x;
                    if stagger && row_index % 2 == 1 {
                        x += gap_x / 2.0;
                    }

                    while x < page_width {
                        points.push((x, y));
                        x += gap_x;
                    }

                    y += gap_y;
                    row_index += 1;
                }

                points
            }
        }
    }
}
