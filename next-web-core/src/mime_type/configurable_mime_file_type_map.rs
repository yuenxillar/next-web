use std::{env, error::Error, fs::File, io::Read, path::Path};

use tracing::error;

use crate::mime_type::mime_type_file::MimeTypeFile;

/// Built-in MIME type table, compiled into the binary.
pub const DEFAULT_MIME_TYPES: &'static str = include_str!("mime.types");

/// A registry of MIME type definitions backed by an ordered list of
/// [`MimeTypeFile`] instances.
///
/// The vector is ordered by priority: index `0` has the highest priority
/// and is consulted first. Additional entries act as fallbacks. The
/// built-in [`DEFAULT_MIME_TYPES`] are always used when no external file
/// is supplied or the supplied path does not exist.
#[derive(Clone)]
pub struct ConfigurableMimeFileTypeMap {
    /// Ordered registries; index 0 wins over later entries.
    mime_types: Vec<MimeTypeFile>,
}

impl ConfigurableMimeFileTypeMap {
    /// Loads MIME type definitions from `mapping_location`.
    ///
    /// Behaviour:
    /// * If `mapping_location` is empty or does not exist, the built-in
    ///   [`DEFAULT_MIME_TYPES`] are parsed instead.
    /// * Otherwise the file at `mapping_location` is read and parsed,
    ///   together with an optional `~/.mime.types` overlay (see
    ///   [`mimetypes_file_type_map`]). The explicitly requested file has
    ///   higher priority than the user-level overlay.
    /// * Any strings in `mappings` are appended to the highest-priority
    ///   registry (index 0). If that registry does not exist, an empty one
    ///   is created first.
    ///
    /// The resulting registries **replace** whatever was stored in
    /// `self.mime_types` before this call.
    pub fn load_file<P>(
        &mut self,
        mapping_location: P,
        mappings: Option<Vec<String>>,
    ) -> Result<(), Box<dyn Error>>
    where
        P: AsRef<str>,
    {
        let mapping_location = mapping_location.as_ref();

        let mut mime_types = if mapping_location.is_empty() || !Path::new(mapping_location).exists()
        {
            // Fall back to the compiled-in defaults.
            mimetypes_file_type_map(DEFAULT_MIME_TYPES)
        } else {
            let mut buf = String::new();
            match File::open(mapping_location) {
                Ok(mut fs) => {
                    // Ignore read errors here; an empty buffer simply yields
                    // an empty registry below.
                    fs.read_to_string(&mut buf).ok();
                    mimetypes_file_type_map(&buf)
                }
                Err(_err) => Default::default(),
            }
        };

        if let Some(mappings) = mappings {
            // Extra mappings come from the caller and are the strongest
            // signal available, so they must override anything coming from
            // the defaults or from files on disk.
            //
            // We achieve that by parsing them into a dedicated registry and
            // inserting it at index 0, the highest-priority slot. Lookups walk
            // the vector in order, so the first match wins and the caller's
            // mappings shadow everything below.
            let mut extra = MimeTypeFile::default();
            for mapping in mappings.into_iter() {
                extra.append_to_registry(mapping.as_str())?;
            }

            // Insert even if `extra` is empty: an empty highest-priority
            // registry is still a meaningful "caller-provided" layer, and it
            // keeps the invariant that index 0 always represents the caller's
            // explicit mappings.
            mime_types.insert(0, extra);
        }

        // Persist the newly built registries; without this the work above
        // would be discarded when the function returns.
        self.mime_types = mime_types;
        Ok(())
    }

    /// Returns the highest-priority registry, if any.
    ///
    /// This is the registry that extra `mappings` were appended to during
    /// the last [`load_file`](Self::load_file) call.
    pub fn primary(&self) -> Option<&MimeTypeFile> {
        self.mime_types.first()
    }

    /// Looks up a MIME type string for `file_ext` across all registries,
    /// returning the first match.
    ///
    /// Registries are searched in priority order, so a mapping in the
    /// explicitly requested file shadows one from `~/.mime.types`.
    pub fn get_mime_type_string(&self, file_ext: &str) -> Option<&str> {
        self.mime_types
            .iter()
            .find_map(|mf| mf.get_mime_type_string(file_ext))
    }

    /// Number of registries currently held.
    pub fn len(&self) -> usize {
        self.mime_types.len()
    }

    /// Returns `true` if no registries are held.
    pub fn is_empty(&self) -> bool {
        self.mime_types.is_empty()
    }
}

impl Default for ConfigurableMimeFileTypeMap {
    fn default() -> Self {
        Self {
            mime_types: Default::default(),
        }
    }
}

/// Builds the ordered registry list for a given MIME source.
///
/// The returned vector has the highest-priority registry at index 0:
/// 1. `content` itself (the explicitly requested or default mime.types).
/// 2. `~/.mime.types`, if present, as a user-level fallback.
///
/// Load failures for `~/.mime.types` are logged and skipped rather than
/// propagated, so a broken user file does not prevent the built-in
/// defaults from working.
fn mimetypes_file_type_map(content: &str) -> Vec<MimeTypeFile> {
    let mut mime_types = Vec::with_capacity(2);

    if let Some(home_dir) = env::home_dir() {
        // Build path: ~/.mime.types
        let mime_path = home_dir.join(".mime.types");

        // Check if the file exists
        if mime_path.exists() {
            // load file
            match MimeTypeFile::from_file(&mime_path) {
                Ok(mf) => mime_types.push(mf),
                Err(err) => error!("Failed to load MIME types from {:?}: {}", mime_path, err),
            };
        }
    }

    match MimeTypeFile::from_str(content) {
        Ok(mf) => mime_types.insert(0, mf),
        Err(err) => error!("Failed to load MIME types from  {}", err),
    };

    mime_types
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_empty() {
        let map = ConfigurableMimeFileTypeMap::default();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
        assert!(map.primary().is_none());
        assert_eq!(map.get_mime_type_string("html"), None);
    }

    #[test]
    fn test_load_default_mime_types() {
        let mut map = ConfigurableMimeFileTypeMap::default();
        // Empty location -> falls back to DEFAULT_MIME_TYPES.
        map.load_file("", None).unwrap();

        assert!(!map.is_empty());
        assert_eq!(map.get_mime_type_string("html"), Some("text/html"));
        assert_eq!(map.get_mime_type_string("htm"), Some("text/html"));
        assert_eq!(map.get_mime_type_string("png"), Some("image/png"));
        assert_eq!(map.get_mime_type_string("svg"), Some("image/svg+xml"));
    }

    #[test]
    fn test_load_file_result_is_stored() {
        // Regression test: load_file must persist the parsed registries
        // in self.mime_types, otherwise lookups return nothing.
        let mut map = ConfigurableMimeFileTypeMap::default();
        map.load_file("", None).unwrap();

        assert!(map.primary().is_some());
        assert!(map.primary().map(|mf| !mf.is_empty()).unwrap_or(false));
    }

    #[test]
    fn test_extra_mappings_are_appended_to_primary() {
        let mut map = ConfigurableMimeFileTypeMap::default();
        map.load_file("", Some(vec!["application/json json".to_string()]))
            .unwrap();

        assert_eq!(map.get_mime_type_string("json"), Some("application/json"));
    }

    #[test]
    fn test_extra_mappings_create_primary_when_missing() {
        let mut map = ConfigurableMimeFileTypeMap::default();
        // A non-existent path with extra mappings still needs a primary
        // registry to append to.
        map.load_file(
            "/definitely/not/a/real/path/mime.types",
            Some(vec!["text/x-custom cus".to_string()]),
        )
        .unwrap();

        assert_eq!(map.get_mime_type_string("cus"), Some("text/x-custom"));
    }

    #[test]
    fn test_load_file_from_disk() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_configurable_mime.types");
        std::fs::write(&temp_file, "text/html html htm\nimage/png png\n").unwrap();

        let mut map = ConfigurableMimeFileTypeMap::default();
        map.load_file(temp_file.to_string_lossy(), None).unwrap();

        assert_eq!(map.get_mime_type_string("html"), Some("text/html"));
        assert_eq!(map.get_mime_type_string("htm"), Some("text/html"));
        assert_eq!(map.get_mime_type_string("png"), Some("image/png"));

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_explicit_file_shadows_defaults() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_shadow_mime.types");
        // Override html to a custom type; html should now resolve to the
        // explicit file's mapping rather than the built-in one.
        std::fs::write(&temp_file, "application/x-html html\n").unwrap();

        let mut map = ConfigurableMimeFileTypeMap::default();
        map.load_file(temp_file.to_string_lossy(), None).unwrap();

        assert_eq!(map.get_mime_type_string("html"), Some("application/x-html"));

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_duplicate_extension_first_wins() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_dup_first_mime.types");
        std::fs::write(&temp_file, "text/plain txt\ntext/html txt\n").unwrap();

        let mut map = ConfigurableMimeFileTypeMap::default();
        map.load_file(temp_file.to_string_lossy(), None).unwrap();

        assert_eq!(map.get_mime_type_string("txt"), Some("text/plain"));

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_add_mapping_in_extra_mappings_overrides() {
        let mut map = ConfigurableMimeFileTypeMap::default();
        map.load_file("", Some(vec!["application/x-custom html".to_string()]))
            .unwrap();

        // add_mapping-style append intentionally overrides the default html.
        assert_eq!(
            map.get_mime_type_string("html"),
            Some("application/x-custom")
        );
    }
}
