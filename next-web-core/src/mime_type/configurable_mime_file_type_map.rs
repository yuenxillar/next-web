use std::{env, fs::File, io::Read, path::Path};

use tracing::error;

use crate::{error::BoxError, mime_type::mime_type_file::MimeTypeFile};

pub const DEFAULT_MIME_TYPES: &'static str = include_str!("mime.types");

#[derive(Clone)]
pub struct ConfigurableMimeFileTypeMap {
    mime_types: Vec<MimeTypeFile>,
}

impl ConfigurableMimeFileTypeMap {
    pub fn load_file<P>(
        &mut self,
        mapping_location: P,
        mappings: Option<Vec<String>>,
    ) -> Result<(), BoxError>
    where
        P: AsRef<str>,
    {
        let mapping_location = mapping_location.as_ref();

        let mut mime_types = if mapping_location.is_empty() || !Path::new(mapping_location).exists()
        {
            mimetypes_file_type_map(DEFAULT_MIME_TYPES)
        } else {
            let mut buf = String::new();
            match File::open(mapping_location) {
                Ok(mut fs) => {
                    fs.read_to_string(&mut buf).ok();
                    mimetypes_file_type_map(&buf)
                }
                Err(_err) => Default::default(),
            }
        };

        if let Some(mappings) = mappings {
            if mime_types.get(0).is_none() {
                mime_types.insert(0, MimeTypeFile::default());
            }

            for mapping in mappings.into_iter() {
                mime_types[0].append_to_registry(mapping.as_str())?;
            }
        }

        Ok(())
    }
}

impl Default for ConfigurableMimeFileTypeMap {
    fn default() -> Self {
        Self {
            mime_types: Default::default(),
        }
    }
}
fn mimetypes_file_type_map(content: &str) -> Vec<MimeTypeFile> {
    let mut mime_types = Vec::with_capacity(2);

    if let Some(home_dir) = env::home_dir() {
        // Build path:~/. mime.types
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
