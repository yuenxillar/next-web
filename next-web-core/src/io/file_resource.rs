use std::{borrow::Cow, fs, io, path::Path, time::SystemTime};

use sha2::Digest;

use crate::io::Resource;

#[derive(Clone)]
pub struct FileResource {
    path: Box<Path>,
    data: Cow<'static, [u8]>,
    metadata: Metadata,
}

impl FileResource {
    pub fn new<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Self::try_from(path.as_ref())
    }

    /// Creates a resource for the given path, reusing the metadata that was
    /// already read for it.
    ///
    /// Reading the metadata of a file costs a system call, so a caller that
    /// already has it, such as the scan of the resources directory, passes it
    /// in instead of letting the resource read it a second time.
    pub(crate) fn from_metadata(path: &Path, metadata: &fs::Metadata) -> io::Result<Self> {
        let data = Cow::from(fs::read(path)?);

        let mut hasher = sha2::Sha256::new();
        hasher.update(&data);
        let hash: [u8; 32] = hasher.finalize().into();

        Ok(FileResource {
            data,
            path: path.into(),
            metadata: Metadata::of(metadata, hash),
        })
    }

    pub fn data(&self) -> &Cow<'static, [u8]> {
        &self.data
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }
}

impl TryFrom<&Path> for FileResource {
    type Error = io::Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        Self::from_metadata(path, &path.symlink_metadata()?)
    }
}

#[derive(Clone)]
pub struct Metadata {
    hash: [u8; 32],
    last_modified: Option<u64>,
    created: Option<u64>,
}

impl Metadata {
    pub fn new(hash: [u8; 32], last_modified: Option<u64>, created: Option<u64>) -> Self {
        Self {
            hash,
            last_modified,
            created,
        }
    }

    /// Creates the metadata of a file from the metadata of the file system and
    /// the hash of its content.
    ///
    /// The last modification and creation times are taken from the file system
    /// metadata, or from the `SOURCE_DATE_EPOCH` environment variable when it
    /// is set, so that a build is reproducible.
    ///
    /// # Arguments
    ///
    /// * `metadata` - The metadata of the file, as read from the file system.
    /// * `hash` - The hash of the content of the file.
    fn of(metadata: &fs::Metadata, hash: [u8; 32]) -> Self {
        let source_date_epoch = match std::env::var("SOURCE_DATE_EPOCH") {
            Ok(value) => value.parse::<u64>().ok(),
            Err(_) => None,
        };

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

        Self::new(
            hash,
            source_date_epoch.or(last_modified),
            source_date_epoch.or(created),
        )
    }

    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }

    pub fn last_modified(&self) -> Option<u64> {
        self.last_modified
    }

    pub fn created(&self) -> Option<u64> {
        self.created
    }
}

impl Resource for FileResource {
    fn get_content(&self) -> io::Result<Cow<'static, [u8]>> {
        Ok(self.data.clone())
    }

    fn content_length(&self) -> u64 {
        self.data.len() as u64
    }

    fn last_modified(&self) -> Option<u64> {
        self.metadata.last_modified
    }

    fn created(&self) -> Option<u64> {
        self.metadata.created
    }

    fn filename(&self) -> Option<&str> {
        self.path.file_name().and_then(|s| s.to_str())
    }

    fn create_relative(&self, relative_path: &str) -> io::Result<Box<dyn Resource>> {
        let parent = self.path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("resource has no parent directory: {}", self.path.display()),
            )
        })?;
        let joined = parent.join(relative_path);

        let canonical_parent = parent.canonicalize()?;
        let canonical_joined = joined.canonicalize()?;
        if !canonical_joined.starts_with(&canonical_parent) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("relative path escapes resource directory: {relative_path}"),
            ));
        }

        Ok(Box::new(Self::try_from(canonical_joined.as_path())?))
    }

    fn path(&self) -> io::Result<&Path> {
        Ok(self.path.as_ref())
    }
}
