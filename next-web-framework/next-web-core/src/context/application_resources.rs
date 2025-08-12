use hashbrown::HashMap;
use std::sync::Arc;
use std::{borrow::Cow, fs, io, path::Path, time::SystemTime};
use tracing::error;

use sha2::Digest;

use crate::constants::application_constants::RESOURCES;

/// Resource files that need to be embedded in binary files
pub trait ResourceLoader: Send + Sync {
    fn load(&self, path: impl AsRef<str>) -> Option<&[u8]>;

    fn load_dir(&self, dir: impl AsRef<str>) -> impl Iterator<Item = &str> + '_;

    fn iter(&self) -> impl Iterator<Item = &str> + '_;
}

#[derive(Clone)]
pub struct ApplicationResources {
    config: Arc<Config>,
    files: Arc<Files>,
}

impl ResourceLoader for ApplicationResources {
    fn load(&self, path: impl AsRef<str>) -> Option<&[u8]> {
        let path = path.as_ref().replace("\\", "/");
        let source = match self.files.inner.get(path.as_str()) {
            Some(source) => source,
            None => return None,
        };

        Some(source.data.as_ref())
    }

    fn load_dir(&self, dir: impl AsRef<str>) -> impl Iterator<Item = &str> + '_ {
        let s1 = dir.as_ref().replace("\\", "/");
        self.files.inner.iter().filter_map(move |(s2, _)| {
            if s2.starts_with(&s1) {
                Some(s2.as_ref())
            } else {
                None
            }
        })
    }

    fn iter(&self) -> impl Iterator<Item = &str> + '_ {
        self.files.inner.iter().map(|(s, _)| s.as_ref())
    }
}

impl Default for ApplicationResources {
    fn default() -> Self {
        let config = Arc::new(Config::default());
        let files = Arc::new(Files::load_file(config.as_ref()));
        Self { config, files }
    }
}

struct Config {
    // pub(super) supported_types: Vec<SupportedTypes>,
    pub(super) maximum_file_size: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // supported_types: Default::default(),
            maximum_file_size: 1024 * 1024 * 10, // 10MB
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum SupportedTypes {
    Json,
    Yaml,
    Xml,
    Toml,
    Html,
    Txt,
    Properties,
    Other(Box<str>),
}

impl From<&str> for SupportedTypes {
    fn from(value: &str) -> Self {
        let value = value.to_lowercase();
        match value.as_str() {
            "json" => SupportedTypes::Json,
            "yaml" | "yml" => SupportedTypes::Yaml,
            "xml" => SupportedTypes::Xml,
            "toml" => SupportedTypes::Toml,
            "html" => SupportedTypes::Html,
            "txt" => SupportedTypes::Txt,
            "properties" => SupportedTypes::Properties,
            _ => SupportedTypes::Other(value.into()),
        }
    }
}

struct Files {
    inner: HashMap<Box<str>, ResourceFile>,
}

impl Files {
    fn load_file(config: &Config) -> Self {
        let mut inner = HashMap::new();
        let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

        let path = Path::new(&dir).join(RESOURCES);

        if !path.exists() {
            error!("Application resources directory not found!")
        } else {
            Self::read(&mut inner, &path, config)
                .map_err(|e| error!("Application resources loading error: {}", e))
                .ok();
        }

        Self { inner }
    }

    fn read(
        map: &mut HashMap<Box<str>, ResourceFile>,
        path: &Path,
        config: &Config,
    ) -> io::Result<()> {
        if path.is_dir() {
            for entry in path.read_dir()? {
                if let Ok(entry) = entry {
                    Self::read(map, &entry.path(), config)?;
                }
            }
        } else {
            if let Some(ext) = path.extension() {
                let ext = ext.to_str().map(|s| s.to_lowercase()).unwrap_or_default();

                match SupportedTypes::from(ext.as_str()) {
                    SupportedTypes::Other(_) => {}
                    _ => {
                        if let Ok(source) = read_file_from_fs(&path, config) {
                            let file_path = path.to_str().unwrap_or_default();
                            if !file_path.is_empty() {
                                // D:\resouces\index.html -> /index.html
                                // /resouces/index.html -> /index.html
                                let s1 = file_path.replace("\\", "/");
                                let key = s1.split(RESOURCES).last().unwrap_or(file_path);

                                map.insert(key.into(), source);
                            }
                        }
                    }
                }
            }
        }

        io::Result::Ok(())
    }
}

pub struct ResourceFile {
    pub data: Cow<'static, [u8]>,
    pub metadata: Metadata,
}

pub struct Metadata {
    pub hash: [u8; 32],
    pub last_modified: Option<u64>,
    pub created: Option<u64>,
}

fn read_file_from_fs(file_path: &Path, config: &Config) -> io::Result<ResourceFile> {
    let metadata = fs::metadata(file_path)?;
    let file_len = metadata.len();
    if file_len > config.maximum_file_size || file_len == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "File size exceeds maximum limit of {}",
                config.maximum_file_size
            ),
        ));
    }

    let data = fs::read(file_path)?;
    let data = Cow::from(data);

    let mut hasher = sha2::Sha256::new();
    hasher.update(&data);
    let hash: [u8; 32] = hasher.finalize().into();

    let source_date_epoch = match std::env::var("SOURCE_DATE_EPOCH") {
        Ok(value) => value.parse::<u64>().ok(),
        Err(_) => None,
    };

    let metadata = fs::metadata(file_path)?;
    let last_modified = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|secs| secs.as_secs());

    let created = metadata
        .created()
        .ok()
        .and_then(|created| created.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|secs| secs.as_secs());

    Ok(ResourceFile {
        data,
        metadata: Metadata {
            hash,
            last_modified: source_date_epoch.or(last_modified),
            created: source_date_epoch.or(created),
        },
    })
}
