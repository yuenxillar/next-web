use std::path::{Path, PathBuf};

#[cfg(feature = "digital-signature")]
use std::fs;

use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

use crate::{PdfResult, error::pdf_error::PdfError, operation::PdfOperation};

#[derive(Debug, Clone)]
pub struct VisibleSignatureOptions {
    pub page: u32,
    pub rect: SignatureRect,
    pub image_path: Option<PathBuf>,
    pub text: Option<String>,
    pub font_name: String,
    pub font_size: f32,
    pub text_color_rgb: (f32, f32, f32),
    pub opacity: f32,
}

impl Default for VisibleSignatureOptions {
    fn default() -> Self {
        Self {
            page: 1,
            rect: SignatureRect::new(360.0, 48.0, 180.0, 72.0),
            image_path: None,
            text: None,
            font_name: "Helvetica".to_string(),
            font_size: 12.0,
            text_color_rgb: (0.15, 0.15, 0.15),
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SignatureRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl SignatureRect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub struct PdfVisibleSignatureOperation {
    pub options: VisibleSignatureOptions,
}

impl PdfVisibleSignatureOperation {
    pub fn new(options: VisibleSignatureOptions) -> Self {
        Self { options }
    }
}

impl PdfOperation<()> for PdfVisibleSignatureOperation {
    fn execute(self, doc: &mut Document) -> PdfResult<()> {
        let pages = doc.get_pages();
        let Some(&page_id) = pages.get(&self.options.page) else {
            return Err(PdfError::InvalidPageNumber(self.options.page));
        };

        let gs_id = add_transparency_group(doc, self.options.opacity)?;
        let image_id = match self.options.image_path.as_deref() {
            Some(path) => Some(add_image_object(doc, path)?),
            None => None,
        };

        let page = doc
            .get_object_mut(page_id)
            .and_then(|obj| obj.as_dict_mut())
            .map_err(|e| PdfError::AnalysisError(format!("Get page {page_id:?} failed: {e}")))?;

        if !page.has(b"Resources") {
            page.set("Resources", Dictionary::new());
        }

        let resources = page
            .get_mut(b"Resources")
            .and_then(|obj| obj.as_dict_mut())
            .map_err(|e| PdfError::AnalysisError(format!("Get page resources failed: {e}")))?;

        register_resource(resources, "ExtGState", "SigAlpha", Object::Reference(gs_id));

        register_resource(
            resources,
            "Font",
            "SigFont",
            Object::Dictionary(Dictionary::from_iter(vec![
                ("Type", "Font".into()),
                ("Subtype", "Type1".into()),
                (
                    "BaseFont",
                    Object::Name(self.options.font_name.as_bytes().to_vec()),
                ),
            ])),
        );

        if let Some(image_id) = image_id {
            register_resource(
                resources,
                "XObject",
                "SigImage",
                Object::Reference(image_id),
            );
        }

        let mut operations = Vec::new();
        operations.push(Operation::new("q", vec![]));
        operations.push(Operation::new(
            "gs",
            vec![Object::Name(b"SigAlpha".to_vec())],
        ));

        if self.options.image_path.is_some() {
            operations.push(Operation::new(
                "q",
                vec![
                    self.options.rect.width.into(),
                    0.into(),
                    0.into(),
                    self.options.rect.height.into(),
                    self.options.rect.x.into(),
                    self.options.rect.y.into(),
                ],
            ));
            operations.push(Operation::new(
                "Do",
                vec![Object::Name(b"SigImage".to_vec())],
            ));
            operations.push(Operation::new("Q", vec![]));
        }

        if let Some(text) = &self.options.text {
            let text_x = self.options.rect.x + 4.0;
            let text_y = self.options.rect.y + (self.options.rect.height / 2.0);

            operations.push(Operation::new("BT", vec![]));
            operations.push(Operation::new(
                "rg",
                vec![
                    self.options.text_color_rgb.0.into(),
                    self.options.text_color_rgb.1.into(),
                    self.options.text_color_rgb.2.into(),
                ],
            ));
            operations.push(Operation::new(
                "Tf",
                vec![
                    Object::Name(b"SigFont".to_vec()),
                    self.options.font_size.into(),
                ],
            ));
            operations.push(Operation::new("Td", vec![text_x.into(), text_y.into()]));
            operations.push(Operation::new(
                "Tj",
                vec![Object::string_literal(text.as_bytes().to_vec())],
            ));
            operations.push(Operation::new("ET", vec![]));
        }

        operations.push(Operation::new("Q", vec![]));

        append_content_stream(doc, page_id, operations)
    }
}

#[derive(Debug, Clone)]
pub enum SignatureCredential {
    Pkcs12 {
        path: PathBuf,
        password: String,
    },
    Pem {
        cert_path: PathBuf,
        key_path: PathBuf,
        key_password: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct DigitalSignOptions {
    pub credential: SignatureCredential,
    pub page: Option<u32>,
    pub reason: Option<String>,
    pub location: Option<String>,
    pub signer_name: Option<String>,
    pub reserve_bytes: usize,
}

impl Default for DigitalSignOptions {
    fn default() -> Self {
        Self {
            credential: SignatureCredential::Pkcs12 {
                path: PathBuf::new(),
                password: String::new(),
            },
            page: Some(1),
            reason: None,
            location: None,
            signer_name: None,
            reserve_bytes: 16_384,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SignedPdf {
    bytes: Vec<u8>,
}

impl SignedPdf {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> PdfResult<()> {
        crate::util::save_bytes(path, &self.bytes)
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(feature = "digital-signature")]
pub struct PdfDigitalSignOperation {
    pub options: DigitalSignOptions,
}

#[cfg(feature = "digital-signature")]
impl PdfDigitalSignOperation {
    pub fn new(options: DigitalSignOptions) -> Self {
        Self { options }
    }
}

#[cfg(feature = "digital-signature")]
impl PdfOperation<SignedPdf> for PdfDigitalSignOperation {
    fn execute(self, doc: &mut Document) -> PdfResult<SignedPdf> {
        sign_document(doc, self.options)
    }
}

#[cfg(not(feature = "digital-signature"))]
pub struct PdfDigitalSignOperation {
    pub options: DigitalSignOptions,
}

#[cfg(not(feature = "digital-signature"))]
impl PdfDigitalSignOperation {
    pub fn new(options: DigitalSignOptions) -> Self {
        Self { options }
    }
}

#[cfg(not(feature = "digital-signature"))]
impl PdfOperation<SignedPdf> for PdfDigitalSignOperation {
    fn execute(self, _doc: &mut Document) -> PdfResult<SignedPdf> {
        let _ = self.options;
        Err(PdfError::FeatureDisabled("digital-signature"))
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
        Ok(Object::Array(arr)) => arr.push(Object::Reference(stream_id)),
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
    dict.set("ca", alpha);
    dict.set("CA", alpha);
    Ok(doc.add_object(Object::Dictionary(dict)))
}

fn add_image_object(doc: &mut Document, path: &Path) -> PdfResult<ObjectId> {
    let img = image::open(path)
        .map_err(|e| PdfError::Io(std::io::Error::other(e.to_string())))?
        .into_rgb8();

    let (width, height) = img.dimensions();
    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name(b"XObject".to_vec()));
    dict.set("Subtype", Object::Name(b"Image".to_vec()));
    dict.set("Width", width);
    dict.set("Height", height);
    dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
    dict.set("BitsPerComponent", 8);

    Ok(doc.add_object(Stream::new(dict, img.into_raw())))
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

#[cfg(feature = "digital-signature")]
fn sign_document(doc: &mut Document, options: DigitalSignOptions) -> PdfResult<SignedPdf> {
    use chrono::Utc;
    use lopdf::Object::String as PdfString;
    use lopdf::StringFormat;

    let mut working = doc.clone();
    let page_id = options
        .page
        .and_then(|page| working.get_pages().get(&page).copied())
        .or_else(|| working.get_pages().values().next().copied())
        .ok_or_else(|| PdfError::SignatureError("document has no pages".to_string()))?;

    let placeholder_contents = vec![0u8; options.reserve_bytes];
    let byte_range_placeholder = BYTE_RANGE_PLACEHOLDER.as_bytes().to_vec();

    let signature_id = working.new_object_id();
    let widget_id = working.new_object_id();

    let signature_dict = Dictionary::from_iter(vec![
        ("Type", "Sig".into()),
        ("Filter", "Adobe.PPKLite".into()),
        ("SubFilter", "adbe.pkcs7.detached".into()),
        (
            "ByteRange",
            Object::String(byte_range_placeholder, StringFormat::Literal),
        ),
        (
            "Contents",
            PdfString(placeholder_contents, StringFormat::Hexadecimal),
        ),
        (
            "M",
            Object::string_literal(format!("D:{}", Utc::now().format("%Y%m%d%H%M%SZ"))),
        ),
    ]);

    working
        .objects
        .insert(signature_id, Object::Dictionary(signature_dict));

    let widget_rect = vec![0.into(), 0.into(), 0.into(), 0.into()];
    let widget = Dictionary::from_iter(vec![
        ("Type", "Annot".into()),
        ("Subtype", "Widget".into()),
        ("FT", "Sig".into()),
        ("Rect", Object::Array(widget_rect)),
        ("V", Object::Reference(signature_id)),
        ("T", Object::string_literal("Signature1")),
        ("F", 4.into()),
        ("P", Object::Reference(page_id)),
    ]);
    working
        .objects
        .insert(widget_id, Object::Dictionary(widget));

    {
        let page = working
            .get_object_mut(page_id)
            .and_then(|obj| obj.as_dict_mut())
            .map_err(|e| PdfError::SignatureError(format!("failed to get signature page: {e}")))?;

        match page.get_mut(b"Annots") {
            Ok(Object::Array(annots)) => annots.push(Object::Reference(widget_id)),
            _ => page.set("Annots", Object::Array(vec![Object::Reference(widget_id)])),
        }
    }

    let root_id = working
        .trailer
        .get(b"Root")
        .and_then(|obj| obj.as_reference())
        .map_err(|e| PdfError::SignatureError(format!("missing catalog root: {e}")))?;

    let existing_acro_form_id = working
        .get_object(root_id)
        .and_then(|obj| obj.as_dict())
        .and_then(|catalog| catalog.get(b"AcroForm"))
        .and_then(|obj| obj.as_reference())
        .ok();

    let acro_form_id = if let Some(id) = existing_acro_form_id {
        id
    } else {
        let created_id = working.add_object(Dictionary::from_iter(vec![
            ("SigFlags", 3.into()),
            ("Fields", Object::Array(Vec::new())),
        ]));
        working
            .get_object_mut(root_id)
            .and_then(|obj| obj.as_dict_mut())
            .map_err(|e| PdfError::SignatureError(format!("failed to get catalog: {e}")))?
            .set("AcroForm", Object::Reference(created_id));
        created_id
    };

    {
        let acro_form = working
            .get_object_mut(acro_form_id)
            .and_then(|obj| obj.as_dict_mut())
            .map_err(|e| PdfError::SignatureError(format!("failed to get AcroForm: {e}")))?;

        acro_form.set("SigFlags", 3);
        match acro_form.get_mut(b"Fields") {
            Ok(Object::Array(fields)) => fields.push(Object::Reference(widget_id)),
            _ => acro_form.set("Fields", Object::Array(vec![Object::Reference(widget_id)])),
        }
    }

    if let Some(reason) = options.reason.as_ref() {
        working
            .get_object_mut(signature_id)
            .and_then(|obj| obj.as_dict_mut())?
            .set("Reason", Object::string_literal(reason.clone()));
    }

    if let Some(location) = options.location.as_ref() {
        working
            .get_object_mut(signature_id)
            .and_then(|obj| obj.as_dict_mut())?
            .set("Location", Object::string_literal(location.clone()));
    }

    if let Some(name) = options.signer_name.as_ref() {
        working
            .get_object_mut(signature_id)
            .and_then(|obj| obj.as_dict_mut())?
            .set("Name", Object::string_literal(name.clone()));
    }

    let mut bytes = crate::util::document_to_bytes(&working)?;

    let contents_marker = format!("<{}>", "00".repeat(options.reserve_bytes));
    let contents_range = find_marker_range(&bytes, &contents_marker)
        .ok_or_else(|| PdfError::SignatureError("signature placeholder not found".to_string()))?;

    let byte_range_pos = find_marker_range(&bytes, BYTE_RANGE_PLACEHOLDER)
        .ok_or_else(|| PdfError::SignatureError("byte range placeholder not found".to_string()))?;

    let byte_range = [
        0usize,
        contents_range.start,
        contents_range.end,
        bytes.len().saturating_sub(contents_range.end),
    ];

    let byte_range_string = format!(
        "[{:010} {:010} {:010} {:010}]",
        byte_range[0], byte_range[1], byte_range[2], byte_range[3]
    );
    if byte_range_string.len() > BYTE_RANGE_PLACEHOLDER.len() {
        return Err(PdfError::SignatureError(
            "byte range placeholder is too small".to_string(),
        ));
    }

    overwrite_marker(
        &mut bytes,
        byte_range_pos.start,
        BYTE_RANGE_PLACEHOLDER.len(),
        &byte_range_string,
    );

    let signed_payload = [&bytes[..contents_range.start], &bytes[contents_range.end..]].concat();
    let der = create_detached_signature_with_openssl_cli(&options.credential, &signed_payload)?;

    let signature_hex = hex::encode(der);
    let max_hex_len = options.reserve_bytes * 2;
    if signature_hex.len() > max_hex_len {
        return Err(PdfError::SignatureError(format!(
            "signature payload exceeds reserved size: {} > {}",
            signature_hex.len(),
            max_hex_len
        )));
    }

    let padded_hex = format!("{signature_hex:0<max_hex_len$}");
    overwrite_marker(&mut bytes, contents_range.start, max_hex_len, &padded_hex);

    Ok(SignedPdf::new(bytes))
}

#[cfg(feature = "digital-signature")]
fn create_detached_signature_with_openssl_cli(
    credential: &SignatureCredential,
    payload: &[u8],
) -> PdfResult<Vec<u8>> {
    use rand::random;

    let temp_root = std::env::temp_dir().join(format!(
        "next-web-pdf-sign-{}-{}",
        std::process::id(),
        random::<u64>()
    ));
    fs::create_dir_all(&temp_root)?;

    let payload_path = temp_root.join("payload.bin");
    let output_path = temp_root.join("signature.der");
    fs::write(&payload_path, payload)?;

    let result = match credential {
        SignatureCredential::Pem {
            cert_path,
            key_path,
            key_password,
        } => sign_with_pem(
            &payload_path,
            &output_path,
            cert_path,
            key_path,
            key_password.as_deref(),
        ),
        SignatureCredential::Pkcs12 { path, password } => {
            let bundle_path = temp_root.join("bundle.pem");
            extract_pkcs12_bundle(path, password, &bundle_path)?;
            sign_with_pem(
                &payload_path,
                &output_path,
                &bundle_path,
                &bundle_path,
                None,
            )
        }
    };

    let der = match result {
        Ok(()) => fs::read(&output_path).map_err(Into::into),
        Err(error) => Err(error),
    };

    let _ = fs::remove_file(&payload_path);
    let _ = fs::remove_file(&output_path);
    let _ = fs::remove_dir_all(&temp_root);

    der
}

#[cfg(feature = "digital-signature")]
fn extract_pkcs12_bundle(path: &Path, password: &str, bundle_path: &Path) -> PdfResult<()> {
    use std::process::Command;

    let output = Command::new("openssl")
        .arg("pkcs12")
        .arg("-in")
        .arg(path)
        .arg("-nodes")
        .arg("-out")
        .arg(bundle_path)
        .arg("-passin")
        .arg(format!("pass:{password}"))
        .output()
        .map_err(|error| {
            PdfError::RuntimeDependencyMissing(format!(
                "failed to launch openssl executable: {error}"
            ))
        })?;

    if output.status.success() {
        return Ok(());
    }

    Err(PdfError::SignatureError(format!(
        "openssl pkcs12 failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

#[cfg(feature = "digital-signature")]
fn sign_with_pem(
    payload_path: &Path,
    output_path: &Path,
    cert_path: &Path,
    key_path: &Path,
    key_password: Option<&str>,
) -> PdfResult<()> {
    use std::process::Command;

    let mut command = Command::new("openssl");
    command
        .arg("cms")
        .arg("-sign")
        .arg("-binary")
        .arg("-md")
        .arg("sha256")
        .arg("-in")
        .arg(payload_path)
        .arg("-signer")
        .arg(cert_path)
        .arg("-inkey")
        .arg(key_path)
        .arg("-outform")
        .arg("DER")
        .arg("-out")
        .arg(output_path);

    if let Some(password) = key_password {
        command.arg("-passin").arg(format!("pass:{password}"));
    }

    let output = command.output().map_err(|error| {
        PdfError::RuntimeDependencyMissing(format!("failed to launch openssl executable: {error}"))
    })?;

    if output.status.success() {
        return Ok(());
    }

    Err(PdfError::SignatureError(format!(
        "openssl cms failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

#[cfg(feature = "digital-signature")]
fn find_marker_range(bytes: &[u8], marker: &str) -> Option<std::ops::Range<usize>> {
    let marker = marker.as_bytes();
    bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .map(|start| start..start + marker.len())
}

#[cfg(feature = "digital-signature")]
fn overwrite_marker(bytes: &mut [u8], start: usize, len: usize, replacement: &str) {
    let mut padded = replacement.as_bytes().to_vec();
    if padded.len() < len {
        padded.extend(std::iter::repeat_n(b' ', len - padded.len()));
    }
    bytes[start..start + len].copy_from_slice(&padded[..len]);
}

#[cfg(feature = "digital-signature")]
const BYTE_RANGE_PLACEHOLDER: &str = "[0000000000 0000000000 0000000000 0000000000]";
