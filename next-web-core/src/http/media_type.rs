use std::collections::HashMap;

use crate::util::{MimeType, MimeTypeUtils};

/// A struct that represents a media type as defined in the HTTP specification.
///
/// This is a subclass of `MimeType` that adds support for quality parameters
/// as defined in the HTTP specification.
///
/// This type is meant to reference media types supported by Spring Framework.
/// If your application or library relies on other media types defined in RFCs,
/// please use `MediaType::parse_media_type` or a custom utility.
///
/// # See also
///
/// * [HTTP 1.1: Semantics and Content, section 3.1.1.1](https://tools.ietf.org/html/rfc7231#section-3.1.1.1)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MediaType {
    /// The underlying MIME type
    mime_type: MimeType,
}

impl MediaType {
    /// Parameter name for quality factor
    const PARAM_QUALITY_FACTOR: &'static str = "q";

    /// A String equivalent of all media ranges.
    pub const ALL_VALUE: &'static str = "*/*";

    /// A String equivalent of `application/atom+xml`.
    pub const APPLICATION_ATOM_XML_VALUE: &'static str = "application/atom+xml";

    /// A String equivalent of `application/cbor`.
    pub const APPLICATION_CBOR_VALUE: &'static str = "application/cbor";

    /// A String equivalent of `application/x-www-form-urlencoded`.
    pub const APPLICATION_FORM_URLENCODED_VALUE: &'static str = "application/x-www-form-urlencoded";

    /// A String equivalent of `application/graphql-response+json`.
    pub const APPLICATION_GRAPHQL_RESPONSE_VALUE: &'static str =
        "application/graphql-response+json";

    /// A String equivalent of `application/json`.
    pub const APPLICATION_JSON_VALUE: &'static str = "application/json";

    /// A String equivalent of `application/octet-stream`.
    pub const APPLICATION_OCTET_STREAM_VALUE: &'static str = "application/octet-stream";

    /// A String equivalent of `application/pdf`.
    pub const APPLICATION_PDF_VALUE: &'static str = "application/pdf";

    /// A String equivalent of `application/problem+json`.
    pub const APPLICATION_PROBLEM_JSON_VALUE: &'static str = "application/problem+json";

    /// A String equivalent of `application/problem+xml`.
    pub const APPLICATION_PROBLEM_XML_VALUE: &'static str = "application/problem+xml";

    /// A String equivalent of `application/x-protobuf`.
    pub const APPLICATION_PROTOBUF_VALUE: &'static str = "application/x-protobuf";

    /// A String equivalent of `application/rss+xml`.
    pub const APPLICATION_RSS_XML_VALUE: &'static str = "application/rss+xml";

    /// A String equivalent of `application/x-ndjson`.
    pub const APPLICATION_NDJSON_VALUE: &'static str = "application/x-ndjson";

    /// A String equivalent of `application/xhtml+xml`.
    pub const APPLICATION_XHTML_XML_VALUE: &'static str = "application/xhtml+xml";

    /// A String equivalent of `application/xml`.
    pub const APPLICATION_XML_VALUE: &'static str = "application/xml";

    /// A String equivalent of `application/yaml`.
    pub const APPLICATION_YAML_VALUE: &'static str = "application/yaml";

    /// A String equivalent of `image/gif`.
    pub const IMAGE_GIF_VALUE: &'static str = "image/gif";

    /// A String equivalent of `image/jpeg`.
    pub const IMAGE_JPEG_VALUE: &'static str = "image/jpeg";

    /// A String equivalent of `image/png`.
    pub const IMAGE_PNG_VALUE: &'static str = "image/png";

    /// A String equivalent of `multipart/form-data`.
    pub const MULTIPART_FORM_DATA_VALUE: &'static str = "multipart/form-data";

    /// A String equivalent of `multipart/mixed`.
    pub const MULTIPART_MIXED_VALUE: &'static str = "multipart/mixed";

    /// A String equivalent of `multipart/related`.
    pub const MULTIPART_RELATED_VALUE: &'static str = "multipart/related";

    /// A String equivalent of `text/event-stream`.
    pub const TEXT_EVENT_STREAM_VALUE: &'static str = "text/event-stream";

    /// A String equivalent of `text/html`.
    pub const TEXT_HTML_VALUE: &'static str = "text/html";

    /// A String equivalent of `text/markdown`.
    pub const TEXT_MARKDOWN_VALUE: &'static str = "text/markdown";

    /// A String equivalent of `text/plain`.
    pub const TEXT_PLAIN_VALUE: &'static str = "text/plain";

    /// A String equivalent of `text/xml`.
    pub const TEXT_XML_VALUE: &'static str = "text/xml";

    /// Media type for "*&#42;*", including all media ranges.

    pub fn all() -> MediaType {
        MediaType::with_subtype(MimeType::WILDCARD_TYPE, MimeType::WILDCARD_TYPE)
    }

    /// Media type for `application/atom+xml`.

    pub fn application_atom_xml() -> MediaType {
        MediaType::with_subtype("application", "atom+xml")
    }

    /// Media type for `application/cbor`.

    pub fn application_cbor() -> MediaType {
        MediaType::with_subtype("application", "cbor")
    }

    /// Media type for `application/x-www-form-urlencoded`.

    pub fn application_form_urlencoded() -> MediaType {
        MediaType::with_subtype("application", "x-www-form-urlencoded")
    }

    /// Media type for `application/graphql-response+json`.

    pub fn application_graphql_response() -> MediaType {
        MediaType::with_subtype("application", "graphql-response+json")
    }

    /// Media type for `application/json`.

    pub fn application_json() -> MediaType {
        MediaType::with_subtype("application", "json")
    }

    /// Media type for `application/octet-stream`.

    pub fn application_octet_stream() -> MediaType {
        MediaType::with_subtype("application", "octet-stream")
    }

    /// Media type for `application/pdf`.

    pub fn application_pdf() -> MediaType {
        MediaType::with_subtype("application", "pdf")
    }

    /// Media type for `application/problem+json`.

    pub fn application_problem_json() -> MediaType {
        MediaType::with_subtype("application", "problem+json")
    }

    /// Media type for `application/problem+xml`.

    pub fn application_problem_xml() -> MediaType {
        MediaType::with_subtype("application", "problem+xml")
    }

    /// Media type for `application/x-protobuf`.

    pub fn application_protobuf() -> MediaType {
        MediaType::with_subtype("application", "x-protobuf")
    }

    /// Media type for `application/rss+xml`.

    pub fn application_rss_xml() -> MediaType {
        MediaType::with_subtype("application", "rss+xml")
    }

    /// Media type for `application/x-ndjson`.

    pub fn application_ndjson() -> MediaType {
        MediaType::with_subtype("application", "x-ndjson")
    }

    /// Media type for `application/xhtml+xml`.

    pub fn application_xhtml_xml() -> MediaType {
        MediaType::with_subtype("application", "xhtml+xml")
    }

    /// Media type for `application/xml`.

    pub fn application_xml() -> MediaType {
        MediaType::with_subtype("application", "xml")
    }

    /// Media type for `application/yaml`.

    pub fn application_yaml() -> MediaType {
        MediaType::with_subtype("application", "yaml")
    }

    /// Media type for `image/gif`.

    pub fn image_gif() -> MediaType {
        MediaType::with_subtype("image", "gif")
    }

    /// Media type for `image/jpeg`.

    pub fn image_jpeg() -> MediaType {
        MediaType::with_subtype("image", "jpeg")
    }

    /// Media type for `image/png`.

    pub fn image_png() -> MediaType {
        MediaType::with_subtype("image", "png")
    }

    /// Media type for `multipart/form-data`.

    pub fn multipart_form_data() -> MediaType {
        MediaType::with_subtype("multipart", "form-data")
    }

    /// Media type for `multipart/mixed`.

    pub fn multipart_mixed() -> MediaType {
        MediaType::with_subtype("multipart", "mixed")
    }

    /// Media type for `multipart/related`.

    pub fn multipart_related() -> MediaType {
        MediaType::with_subtype("multipart", "related")
    }

    /// Media type for `text/event-stream`.
    pub fn text_event_stream() -> MediaType {
        MediaType::with_subtype("text", "event-stream")
    }

    /// Media type for `text/html`.

    pub fn text_html() -> MediaType {
        MediaType::with_subtype("text", "html")
    }

    /// Media type for `text/markdown`.

    pub fn text_markdown() -> MediaType {
        MediaType::with_subtype("text", "markdown")
    }

    /// Media type for `text/plain`.

    pub fn text_plain() -> MediaType {
        MediaType::with_subtype("text", "plain")
    }

    /// Media type for `text/xml`.

    pub fn text_xml() -> MediaType {
        MediaType::with_subtype("text", "xml")
    }

    /// Create a new `MediaType` for the given primary type.
    ///
    /// The subtype is set to `"*"`, parameters empty.
    ///
    /// # Arguments
    ///
    /// * `type_` - the primary type
    ///
    /// # Panics
    ///
    /// Panics if any of the parameters contain illegal characters
    pub fn new(type_: &str) -> Self {
        MediaType {
            mime_type: MimeType::with_subtype(type_, MimeType::WILDCARD_TYPE),
        }
    }

    /// Create a new `MediaType` for the given primary type and subtype.
    ///
    /// The parameters are empty.
    ///
    /// # Arguments
    ///
    /// * `type_` - the primary type
    /// * `subtype` - the subtype
    ///
    /// # Panics
    ///
    /// Panics if any of the parameters contain illegal characters
    pub fn with_subtype(type_: &str, subtype: &str) -> Self {
        MediaType {
            mime_type: MimeType::with_subtype(type_, subtype),
        }
    }

    /// Create a new `MediaType` for the given type, subtype, and character set.
    ///
    /// # Arguments
    ///
    /// * `type_` - the primary type
    /// * `subtype` - the subtype
    /// * `charset` - the character set
    ///
    /// # Panics
    ///
    /// Panics if any of the parameters contain illegal characters
    pub fn with_charset(type_: &str, subtype: &str, charset: &str) -> Self {
        MediaType {
            mime_type: MimeType::with_charset(type_, subtype, charset),
        }
    }

    /// Create a new `MediaType` for the given type, subtype, and quality value.
    ///
    /// # Arguments
    ///
    /// * `type_` - the primary type
    /// * `subtype` - the subtype
    /// * `quality_value` - the quality value (between 0.0 and 1.0)
    ///
    /// # Panics
    ///
    /// Panics if any of the parameters contain illegal characters
    pub fn with_quality(type_: &str, subtype: &str, quality_value: f64) -> Self {
        let mut parameters = HashMap::new();
        parameters.insert(
            Self::PARAM_QUALITY_FACTOR.to_string(),
            quality_value.to_string(),
        );
        MediaType {
            mime_type: MimeType::with_parameters(type_, subtype, parameters),
        }
    }

    /// Copy-constructor that copies the type, subtype and parameters of the given
    /// `MediaType`, and allows to set the specified character set.
    ///
    /// # Arguments
    ///
    /// * `other` - the other media type
    /// * `charset` - the character set
    ///
    /// # Panics
    ///
    /// Panics if any of the parameters contain illegal characters
    pub fn from_other_with_charset(other: &MediaType, charset: &str) -> Self {
        MediaType {
            mime_type: MimeType::with_charset_from(&other.mime_type, charset),
        }
    }

    /// Copy-constructor that copies the type and subtype of the given `MediaType`,
    /// and allows for different parameters.
    ///
    /// # Arguments
    ///
    /// * `other` - the other media type
    /// * `parameters` - the parameters, may be `None`
    ///
    /// # Panics
    ///
    /// Panics if any of the parameters contain illegal characters
    pub fn from_other_with_parameters(
        other: &MediaType,
        parameters: HashMap<String, String>,
    ) -> Self {
        MediaType {
            mime_type: MimeType::with_parameters_from(&other.mime_type, parameters),
        }
    }

    /// Create a new `MediaType` for the given type, subtype, and parameters.
    ///
    /// # Arguments
    ///
    /// * `type_` - the primary type
    /// * `subtype` - the subtype
    /// * `parameters` - the parameters, may be `None`
    ///
    /// # Panics
    ///
    /// Panics if any of the parameters contain illegal characters
    pub fn with_parameters(
        type_: &str,
        subtype: &str,
        parameters: HashMap<String, String>,
    ) -> Self {
        MediaType {
            mime_type: MimeType::with_parameters(type_, subtype, parameters),
        }
    }

    /// Create a new `MediaType` for the given `MimeType`.
    ///
    /// The type, subtype and parameters information is copied and `MediaType`-specific
    /// checks on parameters are performed.
    ///
    /// # Arguments
    ///
    /// * `mime_type` - the MIME type
    ///
    /// # Panics
    ///
    /// Panics if any of the parameters contain illegal characters
    pub fn from_mime_type(mime_type: MimeType) -> Self {
        let media_type = MediaType { mime_type };
        // Validate quality parameter if present
        if let Some(quality) = media_type.get_parameters().get(Self::PARAM_QUALITY_FACTOR) {
            let unquoted = Self::unquote(quality);
            let d: f64 = unquoted.parse().expect(&format!(
                "Invalid quality value \"{}\": should be a number",
                unquoted
            ));
            assert!(
                (0.0..=1.0).contains(&d),
                "Invalid quality value \"{}\": should be between 0.0 and 1.0",
                unquoted
            );
        }
        media_type
    }

    /// Validates parameters for this media type.
    ///
    /// # Arguments
    ///
    /// * `parameter` - the parameter name
    /// * `value` - the parameter value
    ///
    /// # Panics
    ///
    /// Panics if the quality value is invalid
    fn check_parameters(&self, parameter: &str, value: &str) {
        // self.mime_type.check_parameters(parameter, value);
        todo!();

        if Self::PARAM_QUALITY_FACTOR == parameter {
            let unquoted_value = Self::unquote(value);
            let d: f64 = unquoted_value.parse().expect(&format!(
                "Invalid quality value \"{}\": should be a number",
                unquoted_value
            ));
            assert!(
                d >= 0.0 && d <= 1.0,
                "Invalid quality value \"{}\": should be between 0.0 and 1.0",
                unquoted_value
            );
        }
    }

    /// Return the quality factor, as indicated by a `q` parameter, if any.
    /// Defaults to `1.0`.
    ///
    /// # Returns
    ///
    /// The quality factor as double value
    pub fn get_quality_value(&self) -> f64 {
        self.get_parameters()
            .get(Self::PARAM_QUALITY_FACTOR)
            .map(|q| Self::unquote(q).parse().unwrap_or(1.0))
            .unwrap_or(1.0)
    }

    /// Indicates whether this `MediaType` is more specific than the given type.
    ///
    /// 1. if this media type has a quality factor higher than the other,
    ///    then this method returns `true`.
    /// 2. if this media type has a quality factor lower than the other,
    ///    then this method returns `false`.
    /// 3. if this mime type has a wildcard type, and the other does not,
    ///    then this method returns `false`.
    /// 4. if this mime type does not have a wildcard type, and the other does,
    ///    then this method returns `true`.
    /// 5. if the two mime types have identical type and subtype, then the mime
    ///    type with the most parameters is more specific than the other.
    /// 6. Otherwise, this method returns `false`.
    ///
    /// # Arguments
    ///
    /// * `other` - the `MimeType` to be compared
    ///
    /// # Returns
    ///
    /// The result of the comparison
    ///
    /// # See also
    ///
    /// * [HTTP 1.1: Semantics and Content, section 5.3.2](https://tools.ietf.org/html/rfc7231#section-5.3.2)
    pub fn is_more_specific(&self, other: &MimeType) -> bool {
        // if let Some(other_media_type) = other.as_any().downcast_ref::<MediaType>() {
        //     let quality1 = self.get_quality_value();
        //     let quality2 = other_media_type.get_quality_value();
        //     if quality1 > quality2 {
        //         return true;
        //     } else if quality1 < quality2 {
        //         return false;
        //     }
        // }
        // self.mime_type.is_more_specific(other)
        todo!()
    }

    /// Indicates whether this `MediaType` is less specific than the given type.
    ///
    /// # Arguments
    ///
    /// * `other` - the `MimeType` to be compared
    ///
    /// # Returns
    ///
    /// The result of the comparison
    ///
    /// # See also
    ///
    /// * `is_more_specific`
    pub fn is_less_specific(&self, other: &MimeType) -> bool {
        // other.is_more_specific(&self.mime_type)
        todo!()
    }

    /// Indicate whether this `MediaType` includes the given media type.
    ///
    /// For instance, `text/*` includes `text/plain` and `text/html`,
    /// and `application/*+xml` includes `application/soap+xml`, etc.
    /// This method is **not** symmetric.
    ///
    /// # Arguments
    ///
    /// * `other` - the reference media type with which to compare
    ///
    /// # Returns
    ///
    /// `true` if this media type includes the given media type; `false` otherwise
    pub fn includes(&self, other: Option<&MediaType>) -> bool {
        if let Some(other) = other {
            self.mime_type.includes(&other.mime_type)
        } else {
            false
        }
    }

    /// Indicate whether this `MediaType` is compatible with the given media type.
    ///
    /// For instance, `text/*` is compatible with `text/plain`,
    /// `text/html`, and vice versa. In effect, this method is similar to
    /// `includes`, except that it **is** symmetric.
    ///
    /// # Arguments
    ///
    /// * `other` - the reference media type with which to compare
    ///
    /// # Returns
    ///
    /// `true` if this media type is compatible with the given media type;
    /// `false` otherwise
    pub fn is_compatible_with(&self, other: Option<&MediaType>) -> bool {
        if let Some(other) = other {
            self.mime_type.is_compatible_with(Some(&other.mime_type))
        } else {
            false
        }
    }

    /// Return a replica of this instance with the quality value of the given `MediaType`.
    ///
    /// # Arguments
    ///
    /// * `media_type` - the media type to copy the quality value from
    ///
    /// # Returns
    ///
    /// The same instance if the given MediaType doesn't have a quality value,
    /// or a new one otherwise
    pub fn copy_quality_value(&self, media_type: &MediaType) -> MediaType {
        if !media_type
            .get_parameters()
            .contains_key(Self::PARAM_QUALITY_FACTOR)
        {
            return self.clone();
        }
        let mut params = self.get_parameters().clone();
        if let Some(q) = media_type.get_parameters().get(Self::PARAM_QUALITY_FACTOR) {
            params.insert(Self::PARAM_QUALITY_FACTOR.to_string(), q.clone());
        }
        MediaType::from_other_with_parameters(self, params)
    }

    /// Return a replica of this instance with its quality value removed.
    ///
    /// # Returns
    ///
    /// The same instance if the media type doesn't contain a quality value,
    /// or a new one otherwise
    pub fn remove_quality_value(&self) -> MediaType {
        if !self
            .get_parameters()
            .contains_key(Self::PARAM_QUALITY_FACTOR)
        {
            return self.clone();
        }
        let mut params = self.get_parameters().clone();
        params.remove(Self::PARAM_QUALITY_FACTOR);
        MediaType::from_other_with_parameters(self, params)
    }

    /// Returns the primary type.
    pub fn get_type(&self) -> &str {
        self.mime_type.get_type()
    }

    /// Returns the subtype.
    pub fn get_subtype(&self) -> &str {
        self.mime_type.get_subtype()
    }

    /// Returns the parameters.
    pub fn get_parameters(&self) -> &HashMap<String, String> {
        // self.mime_type.get_parameters()

        todo!()
    }

    /// Returns a parameter value by name.
    pub fn get_parameter(&self, name: &str) -> Option<&String> {
        self.mime_type.get_parameter(name)
    }

    /// Returns whether this is a wildcard type.
    pub fn is_wildcard_type(&self) -> bool {
        self.mime_type.is_wildcard_type()
    }

    /// Returns whether this is a wildcard subtype.
    pub fn is_wildcard_subtype(&self) -> bool {
        self.mime_type.is_wildcard_subtype()
    }

    /// Returns whether this is a concrete type (not a wildcard).
    pub fn is_concrete(&self) -> bool {
        self.mime_type.is_concrete()
    }

    /// Unquote a quoted string value.
    fn unquote(value: &str) -> String {
        let trimmed = value.trim();
        if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
            trimmed[1..trimmed.len() - 1].to_string()
        } else {
            trimmed.to_string()
        }
    }

    /// Parse the given String value into a `MediaType` object.
    ///
    /// # Arguments
    ///
    /// * `value` - the string to parse
    ///
    /// # Returns
    ///
    /// The parsed media type
    ///
    /// # Panics
    ///
    /// Panics if the media type value cannot be parsed
    pub fn parse_media_type(media_type: &str) -> MediaType {
        let mime_type = MimeTypeUtils::parse_mime_type(media_type);
        MediaType::from_mime_type(mime_type)
    }

    /// Parse the comma-separated string into a list of `MediaType` objects.
    ///
    /// This method can be used to parse an Accept or Content-Type header.
    ///
    /// # Arguments
    ///
    /// * `media_types` - the string to parse
    ///
    /// # Returns
    ///
    /// The list of media types
    pub fn parse_media_types(media_types: Option<&str>) -> Vec<MediaType> {
        let media_types = match media_types {
            Some(s) if !s.is_empty() => s,
            _ => return Vec::new(),
        };

        let tokenized_types = MimeTypeUtils::tokenize(media_types);
        let mut result = Vec::with_capacity(tokenized_types.len());
        for type_str in &tokenized_types {
            if !type_str.trim().is_empty() {
                result.push(MediaType::parse_media_type(type_str));
            }
        }
        result
    }

    /// Parse the given list of (potentially) comma-separated strings into a
    /// list of `MediaType` objects.
    ///
    /// This method can be used to parse an Accept or Content-Type header.
    ///
    /// # Arguments
    ///
    /// * `media_types` - the list of strings to parse
    ///
    /// # Returns
    ///
    /// The list of media types
    pub fn parse_media_types_from_list(media_types: Option<&[String]>) -> Vec<MediaType> {
        let media_types = match media_types {
            Some(list) if !list.is_empty() => list,
            _ => return Vec::new(),
        };

        if media_types.len() == 1 {
            return MediaType::parse_media_types(Some(&media_types[0]));
        }

        let mut result = Vec::new();
        for media_type in media_types {
            result.extend(MediaType::parse_media_types(Some(media_type)));
        }
        result
    }

    /// Re-create the given mime types as media types.
    ///
    /// # Arguments
    ///
    /// * `mime_types` - the list of mime types
    ///
    /// # Returns
    ///
    /// The list of media types
    pub fn as_media_types(mime_types: &[MimeType]) -> Vec<MediaType> {
        mime_types
            .iter()
            .map(|m| MediaType::as_media_type(m))
            .collect()
    }

    /// Re-create the given mime type as a media type.
    ///
    /// # Arguments
    ///
    /// * `mime_type` - the mime type
    ///
    /// # Returns
    ///
    /// The media type
    pub fn as_media_type(mime_type: &MimeType) -> MediaType {
        // if let Some(media_type) = mime_type.as_any().downcast_ref::<MediaType>() {
        //     return media_type.clone();
        // }
        // MediaType::with_parameters(
        //     mime_type.get_type(),
        //     mime_type.get_subtype(),
        //     Some(mime_type.get_parameters().clone()),
        // )
        todo!()
    }

    /// Return a string representation of the given list of `MediaType` objects.
    ///
    /// This method can be used for an `Accept` or `Content-Type` header.
    ///
    /// # Arguments
    ///
    /// * `media_types` - the media types to create a string representation for
    ///
    /// # Returns
    ///
    /// The string representation
    pub fn to_string(media_types: &[MediaType]) -> String {
        let mime_types: Vec<&MimeType> = media_types.iter().map(|m| &m.mime_type).collect();
        MimeTypeUtils::to_string(mime_types)
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.mime_type)
    }
}
