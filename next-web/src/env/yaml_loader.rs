use std::io;

use serde_yaml::Value;

use next_web_core::{io::Resource, util::indexmap::IndexMap};

/// Loads `.yml` / `.yaml` files into flattened, insertion-ordered maps.
///
/// Each YAML document becomes one `IndexMap<String, String>`:
///
/// - Nested mappings are flattened into dotted keys: `a: { b: 1 }` → `a.b = 1`.
/// - Sequences are flattened into bracketed indices: `a: [x, y]` → `a[0] = x`,
///   `a[1] = y`.
/// - Scalars (strings, numbers, booleans, null) are converted to their string
///   representation.
pub struct YamlLoader<'a> {
    resource: &'a dyn Resource,
}

impl<'a> YamlLoader<'a> {
    /// Create a new loader for the given resource.
    pub fn new(resource: &'a dyn Resource) -> Self {
        Self { resource }
    }

    /// Load all YAML documents, each flattened into an insertion-ordered map.
    pub fn load(&self) -> io::Result<Vec<IndexMap<String, String>>> {
        let bytes = self.resource.get_content()?;

        // serde_yaml supports multi-document streams via `Deserializer`.
        let deserializer = serde_yaml::Deserializer::from_slice(&bytes);

        let mut result = Vec::new();
        for document in deserializer {
            let value = Value::deserialize(document).map_err(to_io_error)?;

            // An empty document (e.g. just `---`) yields Null; skip it.
            if value.is_null() {
                continue;
            }

            let mut map = IndexMap::new();
            flatten(&value, "", &mut map);
            result.push(map);
        }

        Ok(result)
    }
}

use serde::Deserialize;

/// Recursively flatten a YAML value into dotted/bracketed keys.
fn flatten(value: &Value, prefix: &str, out: &mut IndexMap<String, String>) {
    match value {
        Value::Mapping(map) => {
            for (k, v) in map {
                let key = match k {
                    Value::String(s) => s.clone(),
                    other => scalar_to_string(other),
                };
                let new_prefix = join_key(prefix, &key);
                flatten(v, &new_prefix, out);
            }
        }
        Value::Sequence(seq) => {
            for (i, v) in seq.iter().enumerate() {
                let new_prefix = format!("{}[{}]", prefix, i);
                flatten(v, &new_prefix, out);
            }
        }
        // Scalar: store as string.
        scalar => {
            out.insert(prefix.to_string(), scalar_to_string(scalar));
        }
    }
}

/// Join a parent prefix and a child key with a dot, unless the prefix is empty.
fn join_key(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{}.{}", prefix, key)
    }
}

/// Convert a scalar YAML value to its string form.
fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        // Sequences and mappings shouldn't reach here (handled above), but
        // fall back to an empty string rather than panicking.
        Value::Sequence(_) | Value::Mapping(_) | Value::Tagged(_) => String::new(),
    }
}

/// Convert a serde_yaml error into an `io::Error`.
fn to_io_error(err: serde_yaml::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err.to_string())
}
