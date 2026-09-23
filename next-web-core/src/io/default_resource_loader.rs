use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    io,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::SystemTime,
};

use tracing::{debug, error, warn};

use crate::{
    constants::application_constants::RESOURCES,
    io::{ConfigOnlyPolicy, FileResource, Resource, ResourceAccessPolicy, ResourceLoader},
    util::matcher::AntPathMatcher,
};

/// A default implementation of [`ResourceLoader`] that caches resources in memory.
#[derive(Clone)]
pub struct DefaultResourceLoader {
    root: Arc<Path>,
    resource_caches: HashMap<Cow<'static, str>, FileResource>,
    /// Last known modification time of each cached resource, used by
    /// [`refresh`](ResourceLoader::refresh) to detect changed files.
    /// Always kept in sync with `resource_caches`.
    resource_modified_times: HashMap<Cow<'static, str>, SystemTime>,
    access_policy: Vec<Arc<dyn ResourceAccessPolicy>>,
    cache_config: Option<ResourceCacheConfig>,
    /// Running total of the content sizes currently held in the cache.
    current_cache_size: u64,
}

/// The resource loader that is shared by the framework when an application does
/// not configure one of its own.
static SHARED: OnceLock<DefaultResourceLoader> = OnceLock::new();

impl DefaultResourceLoader {
    /// Returns the resource loader that is shared by the framework.
    ///
    /// The loader reads the [`resources`](Self::resources_dir) directory of the
    /// application once, the first time it is used, so the directory is scanned
    /// and its files are read a single time no matter how many components look
    /// a resource up. It is the loader used by the banner and by the config
    /// data when the application does not configure a resource loader of its
    /// own.
    ///
    /// A missing resources directory is not an error: the loader then simply
    /// holds no resource. Use [`new`](Self::new) to load another directory.
    pub fn shared() -> &'static DefaultResourceLoader {
        SHARED.get_or_init(|| {
            let mut resource_loader = DefaultResourceLoader::default();
            // An application without a resources directory is not an error: the
            // locations of its resources are simply empty.
            let _ = resource_loader.load();
            resource_loader
        })
    }

    /// Creates a new default resource loader with the specified root directory and cache configuration.
    pub fn new<P>(root: P, cache_config: Option<ResourceCacheConfig>) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            root: Arc::from(root.as_ref()),
            cache_config,
            ..Self::default()
        }
    }

    /// Create a new default resource loader with the specified cache configuration.
    pub fn with_cache_config(cache_config: ResourceCacheConfig) -> Self {
        Self {
            cache_config: Some(cache_config),
            ..Self::default()
        }
    }

    /// Sets the root directory of the resources.
    pub fn set_root<P>(&mut self, root: P)
    where
        P: AsRef<Path>,
    {
        self.root = Arc::from(root.as_ref());
    }

    /// Returns the root directory of the resources.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Register an additional access policy.
    ///
    /// Every registered policy must allow a location for it to be resolved.
    pub fn add_access_policy<T>(&mut self, policy: T)
    where
        T: ResourceAccessPolicy,
        T: 'static,
    {
        self.access_policy.push(Arc::new(policy));
    }

    /// Return the currently registered access policies.
    pub fn access_policies(&self) -> &[Arc<dyn ResourceAccessPolicy>] {
        &self.access_policy
    }

    /// Clear all resource caches in this resource loader.
    pub fn clear_resource_caches(&mut self) {
        self.resource_caches.clear();
        self.resource_modified_times.clear();
        self.current_cache_size = 0;
    }

    /// Return the cache configuration for this resource loader.
    pub fn cache_config(&self) -> Option<&ResourceCacheConfig> {
        self.cache_config.as_ref()
    }

    /// Resolution order:
    ///
    /// 1. `CARGO_MANIFEST_DIR`, when the binary is launched by Cargo.
    /// 2. The current working directory.
    /// 3. `"."` as a last resort.
    pub fn resources_dir() -> PathBuf {
        let base = std::env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));

        base.join(RESOURCES)
    }

    /// Recursively scan `path`, inserting every supported file into the cache.
    ///
    /// Directory entries are visited in sorted order so that the resulting
    /// cache is deterministic across file systems.
    ///
    /// `visited` holds canonicalized directory paths already scanned, and is
    /// used to break symlink cycles.
    ///
    /// When `skip_unchanged` is `true`, files whose modification time matches
    /// the cached value are skipped, so only new or modified resources are
    /// re-read. The initial load passes `false`; refresh passes `true`.
    fn scan_dir(
        &mut self,
        path: &Path,
        base: &Path,
        visited: &mut HashSet<PathBuf>,
        skip_unchanged: bool,
    ) -> io::Result<()> {
        // Guard against symlink cycles by canonicalizing directories.
        if path.is_dir() {
            let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            if !visited.insert(canonical) {
                warn!("Skipping already visited directory: {}", path.display());
                return Ok(());
            }

            let mut entries: Vec<_> = path.read_dir()?.collect::<Result<Vec<_>, _>>()?;

            // Sort for deterministic load order.
            entries.sort_by_cached_key(|entry| entry.file_name());

            for entry in entries {
                self.scan_dir(&entry.path(), base, visited, skip_unchanged)?;
            }

            return Ok(());
        }

        // Only consider regular files.
        let metadata = path.symlink_metadata()?;
        if !metadata.is_file() {
            return Ok(());
        }

        // Every registered policy must allow the location.
        if self
            .access_policy
            .iter()
            .any(|policy| !policy.is_allowed(&path.to_string_lossy()))
        {
            return Ok(());
        }

        // Derive the cache key before reading, so that unreadable-but-unnamed
        // files do not trigger a pointless read.
        let Some(key) = Self::resource_key(path, base) else {
            warn!("Cannot derive resource key for {}", path.display());
            return Ok(());
        };

        // During a refresh, skip files that have not changed since they were
        // cached. Files whose modification time cannot be read are always
        // re-read.
        if skip_unchanged {
            if let Some(modified) = metadata.modified().ok() {
                if self
                    .resource_modified_times
                    .get(key.as_str())
                    .is_some_and(|cached| *cached == modified)
                {
                    return Ok(());
                }
            }
        }

        // Check the size limits before reading, so oversized files are never
        // loaded into memory. The metadata is reused below for the final size,
        // keeping the limit check and the accounting consistent.
        let file_size = metadata.len();

        if !self.check_cache_limits(file_size) {
            self.handle_overflow(&key, file_size);
            // Drop the stale cached copy, if any, so lookups no longer
            // return outdated content for a resource that does not fit the
            // cache limits anymore.
            self.remove_cached(&key);
            return Ok(());
        }

        // Read the file contents only after the size check passes.
        // The metadata was read above, so the resource reuses it instead of
        // reading it a second time.
        let source = match FileResource::from_metadata(path, &metadata) {
            Ok(source) => source,
            Err(e) => {
                warn!("Skipping unreadable resource {}: {}", path.display(), e);
                self.remove_cached(&key);
                return Ok(());
            }
        };

        let actual_size = source.content_length();
        if let Some(old) = self.resource_caches.insert(Cow::Owned(key.clone()), source) {
            if skip_unchanged {
                debug!("Resource modified, reloading: {}", key);
            } else {
                warn!("Duplicate resource key, overwriting: {}", key);
            }
            self.current_cache_size = self.current_cache_size.saturating_sub(old.content_length());
        }
        self.current_cache_size = self.current_cache_size.saturating_add(actual_size);

        // Remember the modification time so a later refresh can skip this
        // file as long as it does not change on disk.
        if let Ok(modified) = metadata.modified() {
            self.resource_modified_times
                .insert(Cow::Owned(key), modified);
        }

        Ok(())
    }

    /// Derive the resource key from a file path.
    ///
    /// The key is the path relative to the resources directory, with forward
    /// slashes and no leading separator, for example `index.html` or
    /// `messages/zh-CN.properties`.
    fn resource_key(path: &Path, base: &Path) -> Option<String> {
        let relative = path.strip_prefix(base).ok()?;
        let key = relative.to_string_lossy().replace('\\', "/");
        if key.is_empty() {
            return None;
        }
        Some(key)
    }

    /// Check whether a resource of the given size may be cached.
    fn check_cache_limits(&self, size: u64) -> bool {
        let cache_config = match self.cache_config.as_ref() {
            Some(config) => config,
            None => return true,
        };
        if size > cache_config.max_file_size {
            return false;
        }

        if cache_config.max_total_size > 0
            && self.current_cache_size.saturating_add(size) > cache_config.max_total_size
        {
            return false;
        }

        true
    }

    /// Handle a resource that exceeds the cache limits.
    ///
    /// Panics when [`ResourceCacheConfig::panic_on_overflow`] is `true`,
    /// otherwise logs a warning.
    fn handle_overflow(&self, key: &str, size: u64) {
        let cache_config = match self.cache_config.as_ref() {
            Some(config) => config,
            None => return,
        };
        let message = format!(
            "resource exceeds cache limits: {key} ({size} bytes, \
                max_file_size = {}, max_total_size = {})",
            cache_config.max_file_size, cache_config.max_total_size,
        );

        if cache_config.panic_on_overflow {
            panic!("{message}");
        }

        warn!("{message}");
    }

    /// Remove the cached resource with the given key, if present.
    ///
    /// Keeps the size accounting and the modification-time index in sync
    /// with the cache.
    fn remove_cached(&mut self, key: &str) {
        if let Some(old) = self.resource_caches.remove(key) {
            self.current_cache_size = self.current_cache_size.saturating_sub(old.content_length());
        }
        self.resource_modified_times.remove(key);
    }

    /// Re-scan the resources directory and update the cache incrementally:
    /// new files are loaded, modified files are re-read, and unchanged files
    /// are left untouched.
    fn _refresh_resources(&mut self) -> io::Result<()> {
        let resources_dir = self.root.to_owned();

        if !resources_dir.exists() {
            warn!(
                "Application resources directory not found, cannot refresh, {}",
                resources_dir.display()
            );
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "resources directory not found",
            ));
        }

        if !resources_dir.is_dir() {
            warn!(
                "Application resources path is not a directory: {}",
                resources_dir.display()
            );
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "resources path is not a directory",
            ));
        }

        // Track visited directories to guard against symlink cycles.
        let mut visited = HashSet::new();

        if let Err(e) = self.scan_dir(&resources_dir, &resources_dir, &mut visited, true) {
            error!("Application resources refresh error: {}", e);
            return Err(e);
        }

        Ok(())
    }
}

impl ResourceLoader for DefaultResourceLoader {
    fn load(&mut self) -> io::Result<()> {
        let resources_dir = self.root.to_owned();

        if !resources_dir.exists() {
            warn!(
                "Application resources directory not found, using default resources, {}",
                resources_dir.display()
            );
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "resources directory not found",
            ));
        }

        if !resources_dir.is_dir() {
            warn!(
                "Application resources path is not a directory: {}",
                resources_dir.display()
            );
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "resources path is not a directory",
            ));
        }

        // Track visited directories to guard against symlink cycles.
        let mut visited = HashSet::new();

        if let Err(e) = self.scan_dir(&resources_dir, &resources_dir, &mut visited, false) {
            error!("Application resources loading error: {}", e);
            return Err(e);
        }

        Ok(())
    }

    fn refresh(&mut self) -> io::Result<()> {
        debug!("Refreshing resource cache");
        self._refresh_resources()
    }

    fn clear(&mut self) {
        debug!("Clearing resource cache");
        self.resource_caches.clear();
        self.resource_modified_times.clear();
        self.current_cache_size = 0;
    }

    fn get_resource(&self, location: &str) -> io::Result<&dyn Resource> {
        // Normalize separators so that Windows-style paths are handled uniformly.
        let normalized: Cow<'_, str> = if location.contains('\\') {
            Cow::Owned(location.replace('\\', "/"))
        } else {
            Cow::Borrowed(location)
        };
        if normalized.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "location is empty",
            ));
        }

        // Return a borrowed view of the cached resource.
        match self.resource_caches.get(normalized.as_ref()) {
            Some(resource) => Ok(resource),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "resource not found",
            )),
        }
    }

    fn get_resources(&self, pattern: &str) -> io::Result<Vec<&dyn Resource>> {
        debug!("Getting resources matching pattern: {}", pattern);

        // Normalize separators so that Windows-style paths are handled uniformly.
        let normalized = pattern.replace('\\', "/");
        if normalized.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "pattern is empty",
            ));
        }

        let matcher = AntPathMatcher::default();

        // Collect the matching keys first and sort them, so the result order
        // is deterministic across cache mutations.
        let mut keys: Vec<&Cow<'static, str>> = self
            .resource_caches
            .keys()
            .filter(|key| matcher.matches(&normalized, key))
            .collect();
        keys.sort();

        Ok(keys
            .into_iter()
            .filter_map(|key| self.resource_caches.get(key))
            .map(|resource| resource as &dyn Resource)
            .collect())
    }

    fn get_directory(&self, location: &str) -> Vec<Cow<'static, str>> {
        let normalized = location.replace('\\', "/");
        let normalized = normalized.trim_matches('/');

        self.resource_caches
            .keys()
            .filter(|key| {
                // A child of `normalized` starts with `normalized/` and has no
                // further `/` after that prefix.
                if let Some(rest) = key.strip_prefix(&normalized) {
                    let rest = rest.strip_prefix('/').unwrap_or(rest);
                    !rest.is_empty() && !rest.contains('/')
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }

    fn paths(&self) -> Vec<Cow<'static, str>> {
        self.resource_caches.keys().cloned().collect()
    }

    fn exists(&self, location: &str) -> bool {
        let normalized = location.replace('\\', "/");
        self.resource_caches.contains_key(normalized.as_str())
    }
}

impl Default for DefaultResourceLoader {
    fn default() -> Self {
        Self {
            root: Arc::from(Self::resources_dir()),
            resource_caches: HashMap::new(),
            resource_modified_times: HashMap::new(),
            access_policy: vec![Arc::new(ConfigOnlyPolicy::new())],
            cache_config: Some(ResourceCacheConfig::default()),
            current_cache_size: 0,
        }
    }
}

/// Caching configuration for [`DefaultResourceLoader`].
#[derive(Debug, Clone)]
pub struct ResourceCacheConfig {
    /// Maximum size, in bytes, for a single resource to be cached.
    ///
    /// Resources whose content length exceeds this limit are returned to the
    /// caller but are not stored in the cache.
    pub max_file_size: u64,

    /// Maximum total size, in bytes, across all cached resources.
    ///
    /// A value of `0` disables the total-size limit.
    pub max_total_size: u64,

    /// Whether exceeding a size limit should panic.
    ///
    /// When `true`, a resource larger than [`max_file_size`](Self::max_file_size)
    /// (or one that would push the total over
    /// [`max_total_size`](Self::max_total_size)) causes a panic. When `false`,
    /// the resource is skipped and not cached.
    pub panic_on_overflow: bool,
}

impl Default for ResourceCacheConfig {
    fn default() -> Self {
        Self {
            // Default: 5 MiB per file.
            max_file_size: 1024 * 1024 * 5,
            // Default: 64 MiB in total.
            max_total_size: 64 * 1024 * 1024,
            // Skipping is the safe default for production.
            panic_on_overflow: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a unique temporary directory for the duration of a test.
    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "next-web-core-resource-loader-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Write `content` to `root/relative`, creating parent directories.
    fn write_file(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn get_resources_matches_wildcard_patterns() {
        let root = temp_root("get-resources");
        write_file(&root, "application.properties", "a=1");
        write_file(&root, "messages/zh-CN.properties", "b=2");
        write_file(&root, "messages/en.properties", "c=3");
        write_file(&root, "static/index.html", "<html/>");

        let mut loader = DefaultResourceLoader::new(&root, None);
        loader.load().unwrap();

        // `*` matches within a single path segment.
        let matched = loader.get_resources("*.properties").unwrap();
        let mut names: Vec<_> = matched
            .iter()
            .map(|resource| resource.filename().unwrap().to_string())
            .collect();
        names.sort();
        assert_eq!(names, vec!["application.properties"]);

        // `**` matches across path segments, including none.
        let matched = loader.get_resources("**/*.properties").unwrap();
        let mut names: Vec<_> = matched
            .iter()
            .map(|resource| resource.filename().unwrap().to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "application.properties",
                "en.properties",
                "zh-CN.properties"
            ]
        );

        // A leading separator in the pattern is tolerated.
        assert_eq!(loader.get_resources("/static/*").unwrap().len(), 1);

        // An exact (non-wildcard) location resolves to itself.
        assert_eq!(loader.get_resources("static/index.html").unwrap().len(), 1);

        // An empty pattern is rejected.
        assert!(loader.get_resources("   ").is_err());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn shares_a_single_loader() {
        let first = DefaultResourceLoader::shared();
        let second = DefaultResourceLoader::shared();

        // The same instance is handed out to every caller, so the resources of
        // the application are read and held once.
        assert!(std::ptr::eq(first, second));
        assert_eq!(
            first.root(),
            DefaultResourceLoader::resources_dir().as_path()
        );
    }
}
