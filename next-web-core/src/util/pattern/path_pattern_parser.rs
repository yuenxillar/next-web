use std::sync::{Arc, LazyLock};

use crate::http::server::PathOptions;

/// Default shared instance that cannot be modified after creation.
static DEFAULT_INSTANCE: LazyLock<Arc<PathPatternParser>> =
    LazyLock::new(|| Arc::new(PathPatternParser::default()));

#[derive(Debug, Clone)]
pub struct PathPatternParser {
    case_sensitive: bool,
    path_options: PathOptions,
}

impl PathPatternParser {
    pub fn default_instance() -> Arc<PathPatternParser> {
        DEFAULT_INSTANCE.clone()
    }

    pub fn set_case_sensitive(&mut self, case_sensitive: bool) {
        self.case_sensitive = case_sensitive;
    }

    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    pub fn set_path_options(&mut self, path_options: PathOptions) {
        self.path_options = path_options;
    }

    pub fn get_path_options(&self) -> PathOptions {
        self.path_options
    }

    /// Prepends "/" if the pattern is non-empty and does not start with "/".
    pub fn init_full_path_pattern(&self, pattern: &str) -> String {
        if !pattern.is_empty() && !pattern.starts_with('/') {
            format!("/{}", pattern)
        } else {
            pattern.to_string()
        }
    }

    // /// Parses the given path pattern into a `PathPattern`.
    // pub fn parse(&self, path_pattern: &str) -> Result<PathPattern, PatternParseException> {
    //     let internal_parser = InternalPathPatternParser::new(self);
    //     internal_parser.parse(path_pattern)
    // }
}

impl Default for PathPatternParser {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            path_options: PathOptions::HTTP_PATH,
        }
    }
}
