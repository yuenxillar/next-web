use next_web_core::{env::PropertySource, util::indexmap::IndexMap};

/// A [`PropertySource`] backed by a map, with origin tracking enabled.
///
/// "Origin tracking" means each property value carries metadata about where it
/// came from (file, line number, etc.), which is useful for diagnostics.
#[derive(Debug)]
pub struct MapPropertySource {
    name: String,
    properties: IndexMap<String, String>,
}

impl MapPropertySource {
    /// Create a new property source.
    pub fn new(name: String, properties: IndexMap<String, String>) -> Self {
        Self { name, properties }
    }
}

impl PropertySource<IndexMap<String, String>> for MapPropertySource {
    fn name(&self) -> &str {
        &self.name
    }

    fn property(&self, key: &str) -> Option<String> {
        self.properties.get(key).map(ToOwned::to_owned)
    }

    fn contains_property(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }

    fn property_names(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    fn source(&self) -> &IndexMap<String, String> {
        &self.properties
    }
}
