use std::io;

use next_web_core::{env::PropertySource, io::Resource, util::indexmap::IndexMap};

use crate::env::{MapPropertySource, PropertiesLoader, PropertySourceLoader};

/// Strategy to load `.properties` files into a [`PropertySource`].
///
/// Supports both the classic `key=value` properties format and the XML
/// properties variant (`.xml`). A single file may contain multiple documents
/// (for example, when the underlying loader splits on a delimiter); each
/// document becomes its own `PropertySource`, suffixed with a document number
/// when there is more than one.
#[derive(Debug, Clone, Default)]
pub struct PropertiesPropertySourceLoader;

impl PropertiesPropertySourceLoader {
    /// The file extension used for XML-format properties files.
    const XML_FILE_EXTENSION: &'static str = ".xml";

    /// Load the raw properties maps from the resource.
    ///
    /// XML files are delegated to the XML properties loader; everything else
    /// is treated as a classic properties file, which may yield multiple
    /// documents.
    fn load_properties(
        &self,
        resource: &dyn Resource,
    ) -> io::Result<Vec<IndexMap<String, String>>> {
        let mut result = Vec::new();

        let is_xml = resource
            .filename()
            .map(|f| f.ends_with(Self::XML_FILE_EXTENSION))
            .unwrap_or(false);

        if is_xml {
            let map = load_xml_properties(resource)?;
            result.push(map);
        } else {
            let documents = PropertiesLoader::new(resource).load()?;
            documents
                .into_iter()
                .for_each(|d| result.push(d.into_map()));
        }

        Ok(result)
    }
}

impl PropertySourceLoader for PropertiesPropertySourceLoader {
    fn file_extensions(&self) -> &[&'static str] {
        &["properties", "xml"]
    }

    fn load(
        &self,
        name: &str,
        resource: &dyn Resource,
    ) -> io::Result<Vec<Box<dyn PropertySource<IndexMap<String, String>>>>> {
        let properties = self.load_properties(resource)?;
        if properties.is_empty() {
            return Ok(Vec::new());
        }

        let mut property_sources: Vec<Box<dyn PropertySource<IndexMap<String, String>>>> =
            Vec::with_capacity(properties.len());

        let properties_len_was_greater_than_one = properties.len() != 1;
        for (i, map) in properties.into_iter().enumerate() {
            let document_number = if properties_len_was_greater_than_one {
                format!(" (document #{})", i)
            } else {
                String::new()
            };
            property_sources.push(Box::new(MapPropertySource::new(
                format!("{}{}", name, document_number),
                map,
            )));
        }
        Ok(property_sources)
    }
}

use quick_xml::events::Event;
use quick_xml::Reader;

/// Load properties from an XML-format properties file.
///
/// The expected structure is:
///
/// ```xml
/// <?xml version="1.0" encoding="UTF-8"?>
/// <!DOCTYPE properties SYSTEM "http://java.sun.com/dtd/properties.dtd">
/// <properties>
///     <comment>ignored</comment>
///     <entry key="foo">bar</entry>
/// </properties>
/// ```
///
/// `<comment>` elements are skipped. Each `<entry>` element contributes one
/// key/value pair, where the value is the element's text content. Entity
/// references such as `&amp;` and `&lt;` are resolved. The returned map
/// preserves insertion order.
fn load_xml_properties(resource: &dyn Resource) -> io::Result<IndexMap<String, String>> {
    let bytes = resource.get_content()?;
    let mut reader = Reader::from_reader(bytes.as_ref());
    reader.config_mut().trim_text(true);

    let mut result = IndexMap::new();
    let mut buf = Vec::new();

    // State for the currently open <entry> element.
    let mut current_key: Option<String> = None;
    let mut in_entry = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                let name = local_name.as_ref();

                if name == b"entry" {
                    in_entry = true;
                    current_key = extract_key_attr(&e)?;
                }
                // <properties>, <comment> and any other elements: ignore.
            }

            Ok(Event::Text(t)) => {
                if in_entry {
                    if let Some(key) = current_key.take() {
                        let value = String::from_utf8_lossy(t.as_ref()).to_string();
                        result.insert(key, value);
                    }
                }
                // Text outside <entry> (e.g. whitespace, comment content): ignore.
            }

            Ok(Event::CData(t)) => {
                if in_entry {
                    if let Some(key) = current_key.take() {
                        let value = String::from_utf8_lossy(t.as_ref()).to_string();
                        result.insert(key, value);
                    }
                }
            }

            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"entry" {
                    in_entry = false;
                    current_key = None;
                }
            }

            Ok(Event::Eof) => break,

            Ok(_) => {} // Decl, DocType, Comment, PI, etc.: ignore

            Err(err) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("failed to parse XML properties: {}", err),
                ));
            }
        }

        buf.clear();
    }

    Ok(result)
}

/// Extract the `key` attribute from an `<entry>` start element.
fn extract_key_attr(e: &quick_xml::events::BytesStart<'_>) -> io::Result<Option<String>> {
    for attr in e.attributes() {
        let attr =
            attr.map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
        if attr.key.local_name().as_ref() == b"key" {
            let value = attr
                .unescape_value()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}
