use std::collections::HashMap;
use std::sync::Arc;
use std::{borrow::Cow, fs, io, path::Path, time::SystemTime};
use tracing::{error, warn};

use sha2::Digest;

use crate::constants::application_constants::RESOURCES;

#[cfg(feature = "embed-resources")]
pub static RESOURCE_LOADER: std::sync::OnceLock<Arc<dyn ResourceLoader>> =
    std::sync::OnceLock::new();

/// Resource files that need to be embedded in binary files
pub trait ResourceLoader
where
    Self: Send + Sync,
{
    fn load(&self, path: &str) -> Option<Cow<'static, [u8]>>;

    fn load_dir(&self, dir: &str) -> Vec<Cow<'static, str>>;

    fn iter(&self) -> Vec<Cow<'static, str>>;
}

#[derive(Clone)]
pub struct ApplicationResources {
    config: Option<Arc<Config>>,
    #[allow(unused)]
    files: Option<Arc<Files>>,
}

impl ApplicationResources {
    pub fn config(&self) -> Option<&Config> {
        self.config.as_deref()
    }
}

impl ResourceLoader for ApplicationResources {
    fn load(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        let path = path.replace("\\", "/");

        #[cfg(feature = "embed-resources")]
        {
            let resource_loader = RESOURCE_LOADER.get()?;
            return resource_loader.load(&path);
        }

        #[cfg(not(feature = "embed-resources"))]
        {
            return self
                .files
                .as_ref()
                .map(|fs| fs.inner.get(path.as_str()))?
                .map(|f| f.data.clone());
        }
    }

    fn load_dir(&self, dir: &str) -> Vec<Cow<'static, str>> {
        let s1 = dir.replace("\\", "/");

        #[cfg(feature = "embed-resources")]
        {
            let resource_loader = match RESOURCE_LOADER.get() {
                Some(resource_loader) => resource_loader,
                None => return Default::default(),
            };

            return resource_loader.load_dir(&s1);
        }

        #[cfg(not(feature = "embed-resources"))]
        {
            return self
                .files
                .as_ref()
                .map(|fs| {
                    fs.inner
                        .iter()
                        .filter_map(|(s2, _)| s2.starts_with(&s1).then(|| s2.clone()))
                        .collect()
                })
                .unwrap_or_default();
        }
    }

    fn iter(&self) -> Vec<Cow<'static, str>> {
        #[cfg(feature = "embed-resources")]
        {
            let resource_loader = match RESOURCE_LOADER.get() {
                Some(resource_loader) => resource_loader,
                None => return Default::default(),
            };

            return resource_loader.iter();
        }

        #[cfg(not(feature = "embed-resources"))]
        {
            return self
                .files
                .as_ref()
                .map(|fs| fs.inner.iter().map(|(s, _)| s.clone()).collect())
                .unwrap_or_default();
        }
    }
}

impl Default for ApplicationResources {
    fn default() -> Self {
        #[cfg(not(feature = "embed-resources"))]
        {
            let config = Arc::new(Config::default());
            let files = Arc::new(Files::load_file(config.as_ref()));
            Self {
                config: Some(config),
                files: Some(files),
            }
        }

        #[cfg(feature = "embed-resources")]
        {
            Self {
                config: None,
                files: None,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    // pub(super) supported_types: Vec<SupportedTypes>,
    pub maximum_file_size: u64,
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

#[allow(unused)]
struct Files {
    inner: HashMap<Cow<'static, str>, ResourceFile>,
}

impl Files {
    #[allow(unused)]
    fn load_file(config: &Config) -> Self {
        let mut inner = HashMap::new();
        let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or(
            std::env::current_dir()
                .map(|dir| dir.to_str().map(ToString::to_string).unwrap_or_default())
                .unwrap_or_default(),
        );

        let path = Path::new(&dir).join(RESOURCES);

        if !path.exists() {
            warn!(
                "Application resources directory not found, using default resources, {}",
                path.display()
            );
        } else {
            Self::read_data(&mut inner, &path, config)
                .map_err(|e| error!("Application resources loading error: {}", e))
                .ok();
        }

        Self { inner }
    }

    #[allow(unused)]
    fn read_data(
        map: &mut HashMap<Cow<'static, str>, ResourceFile>,
        path: &Path,
        config: &Config,
    ) -> io::Result<()> {
        if path.is_dir() {
            for entry in path.read_dir()? {
                if let Ok(entry) = entry {
                    Self::read_data(map, &entry.path(), config)?;
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
                                // D:\resouces\index.html -> index.html
                                // resouces/index.html -> index.html
                                let s1 = file_path.replace("\\", "/");
                                let key = s1.split(RESOURCES).last().unwrap_or(file_path);

                                map.insert(Cow::Owned(key.into()), source);
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
