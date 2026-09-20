use std::io;

use next_web_core::{env::PropertySource, util::indexmap::IndexMap};

use crate::env::{MapPropertySource, PropertySourceLoader, YamlLoader};

/// Strategy to load `.yml` (or `.yaml`) files into a [`PropertySource`].
///
/// Each YAML document in the resource becomes its own property source. When
/// there is more than one document, the source name is suffixed with a
/// document number to keep names unique.
#[derive(Debug, Clone, Default)]
pub struct YamlPropertySourceLoader;

impl PropertySourceLoader for YamlPropertySourceLoader {
    fn file_extensions(&self) -> &[&'static str] {
        &["yml", "yaml"]
    }

    fn load(
        &self,
        name: &str,
        resource: &dyn next_web_core::io::Resource,
    ) -> io::Result<Vec<Box<dyn PropertySource<IndexMap<String, String>>>>> {
        // In the original this is a runtime classpath probe. In Rust the
        // equivalent concern is handled at compile time via a feature flag,
        // so no runtime check is needed here.
        let loaded = YamlLoader::new(resource).load()?;

        if loaded.is_empty() {
            return Ok(Vec::new());
        }

        let document_count = loaded.len();
        let mut property_sources: Vec<Box<dyn PropertySource<IndexMap<String, String>>>> =
            Vec::with_capacity(document_count);

        for (i, map) in loaded.into_iter().enumerate() {
            let document_number = if document_count != 1 {
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
