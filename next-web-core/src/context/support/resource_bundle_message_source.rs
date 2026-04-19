use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use arc_swap::ArcSwap;
use encoding::label::encoding_from_whatwg_label;
use tracing::warn;

use crate::constants::application_constants::MESSAGES;
use crate::context::MessageSource;
use crate::context::application_resources::ResourceLoader;
use crate::util::locale::Locale;

const PROPERTIES_EXTENSION: &str = "properties";

#[derive(Debug, Clone)]
struct MessageTemplate {
    pattern: String,
    segments: Vec<TemplateSegment>,
}

#[derive(Debug, Clone)]
enum TemplateSegment {
    Literal {
        start: usize,
        end: usize,
    },
    Argument {
        index: usize,
        start: usize,
        end: usize,
    },
}

#[derive(Debug)]
pub struct ResourceBundle {
    basename: String,
    locale: Locale,
    messages: HashMap<String, String>,
    templates: StdRwLock<HashMap<String, MessageTemplate>>,
}

impl ResourceBundle {
    fn new(basename: String, locale: Locale, messages: HashMap<String, String>) -> Self {
        Self {
            basename,
            locale,
            messages,
            // Templates are parsed lazily because many lookups only need the raw string.
            // This avoids paying the formatting setup cost for every message during preload.
            templates: StdRwLock::new(HashMap::new()),
        }
    }

    pub fn basename(&self) -> &str {
        &self.basename
    }

    pub fn locale(&self) -> Locale {
        self.locale
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.messages.get(key).cloned()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.messages.contains_key(key)
    }

    pub fn format(&self, key: &str, args: &[impl AsRef<str>]) -> Option<String> {
        {
            let templates = self
                .templates
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(template) = templates.get(key) {
                return Some(render_message_template(template, args));
            }
        }

        let pattern = self.messages.get(key)?;
        let parsed = parse_message_template(pattern);

        let mut templates = self
            .templates
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let template = templates.entry(key.to_string()).or_insert(parsed);
        Some(render_message_template(template, args))
    }
}

#[derive(Debug, Clone)]
struct CachedBundle {
    bundle: Arc<ResourceBundle>,
    loaded_at_millis: i64,
}

pub struct ResourceBundleMessageSource {
    basenames: Vec<String>,
    default_encoding: Option<String>,
    cache_millis: i64,
    fallback_to_system_locale: bool,
    default_locale: Option<Locale>,
    resource_loader: Arc<dyn ResourceLoader>,
    cached_resource_bundles: ArcSwap<HashMap<String, HashMap<Locale, CachedBundle>>>,
}

impl ResourceBundleMessageSource {
    pub fn new(resource_loader: Arc<dyn ResourceLoader>) -> Self {
        Self {
            basenames: Vec::new(),
            default_encoding: Some("UTF-8".to_string()),
            cache_millis: -1,
            fallback_to_system_locale: true,
            default_locale: None,
            resource_loader,
            cached_resource_bundles: ArcSwap::new(Arc::new(HashMap::default())),
        }
    }

    pub fn basenames(&self) -> &[String] {
        &self.basenames
    }

    pub fn set_basenames(&mut self, basenames: Vec<String>) {
        let values = basenames
            .into_iter()
            .filter(|basename| !basename.trim().is_empty())
            .collect::<Vec<_>>();

        assert!(
            !values.is_empty(),
            "At least one basename is required for message resolution"
        );

        self.basenames = values;
        self.clear_cache();
    }

    pub fn add_basename(&mut self, basename: impl Into<String>) {
        let basename = basename.into();
        if basename.trim().is_empty() {
            return;
        }

        if !self.basenames.iter().any(|existing| existing == &basename) {
            self.basenames.push(basename);
            self.clear_cache();
        }
    }

    pub fn default_encoding(&self) -> Option<&str> {
        self.default_encoding.as_deref()
    }

    pub fn set_default_encoding(&mut self, encoding: impl Into<String>) {
        self.default_encoding = Some(encoding.into());
        self.clear_cache();
    }

    pub fn clear_default_encoding(&mut self) {
        self.default_encoding = None;
        self.clear_cache();
    }

    pub fn cache_millis(&self) -> i64 {
        self.cache_millis
    }

    pub fn set_cache_millis(&mut self, millis: i64) {
        self.cache_millis = millis;
        self.clear_cache();
    }

    pub fn set_cache_seconds(&mut self, seconds: i64) {
        self.set_cache_millis(seconds.saturating_mul(1000));
    }

    pub fn default_locale(&self) -> Option<Locale> {
        self.default_locale
    }

    pub fn set_default_locale(&mut self, locale: Option<Locale>) {
        self.default_locale = locale;
        self.clear_cache();
    }

    pub fn fallback_to_system_locale(&self) -> bool {
        self.fallback_to_system_locale
    }

    pub fn set_fallback_to_system_locale(&mut self, fallback: bool) {
        self.fallback_to_system_locale = fallback;
        self.clear_cache();
    }

    pub fn resource_loader(&self) -> &Arc<dyn ResourceLoader> {
        &self.resource_loader
    }

    pub fn resolve_code_without_arguments(&self, code: &str, locale: &Locale) -> Option<String> {
        for basename in self.basenames.iter() {
            if let Some(bundle) = self.get_resource_bundle(basename, locale) {
                if let Some(result) = bundle.get_string(code) {
                    return Some(result);
                }
            }
        }

        None
    }

    pub fn resolve_code(
        &self,
        code: &str,
        locale: &Locale,
        args: &[impl AsRef<str>],
    ) -> Option<String> {
        for basename in &self.basenames {
            if let Some(bundle) = self.get_resource_bundle(basename, locale) {
                if let Some(result) = bundle.format(code, args) {
                    return Some(result);
                }
            }
        }

        None
    }

    pub fn clear_cache(&self) {
        self.cached_resource_bundles
            .store(Arc::new(Default::default()));
    }

    pub fn preload_all(&self) -> io::Result<()> {
        let system_locale = self.fallback_to_system_locale.then(Locale::locale);

        for basename in self.basenames.iter() {
            let discovered = self.discover_bundle_locales(basename);
            if discovered.is_empty() {
                continue;
            }

            let has_default_bundle = discovered.contains(&None);
            let mut target_locales: HashSet<Locale> = discovered.into_iter().flatten().collect();

            if has_default_bundle {
                target_locales.extend(Locale::all_locales());
            }

            if let Some(default_locale) = self.default_locale {
                target_locales.insert(default_locale);
            }

            if let Some(system_locale) = system_locale {
                target_locales.insert(system_locale);
            }

            for locale in target_locales {
                let _ = self.try_get_resource_bundle(basename, &locale, false);
            }
        }

        Ok(())
    }

    pub fn get_basename_set(&self) -> &[String] {
        &self.basenames
    }

    fn get_resource_bundle(&self, basename: &str, locale: &Locale) -> Option<Arc<ResourceBundle>> {
        self.try_get_resource_bundle(basename, locale, true)
    }

    fn try_get_resource_bundle(
        &self,
        basename: &str,
        locale: &Locale,
        log_missing: bool,
    ) -> Option<Arc<ResourceBundle>> {
        if let Some(bundle) = self.get_cached_bundle(basename, locale) {
            return Some(bundle);
        }

        let resolved = self
            .load_bundle_hierarchy(basename, locale)
            .inspect_err(|error| {
                if log_missing {
                    warn!(
                        "ResourceBundle [{}] not found for locale [{}]: {}",
                        basename,
                        locale.as_str(),
                        error
                    );
                }
            })
            .ok()?;

        let loaded_at = current_time_millis();
        let bundle = Arc::new(ResourceBundle::new(basename.to_string(), *locale, resolved));
        let cache = self.cached_resource_bundles.load();

        if let Some(bundle) = self.cached_bundle_from_map(&cache, basename, locale) {
            return Some(bundle);
        }

        self.cached_resource_bundles.rcu(|cache| {
            let mut map = HashMap::clone(&cache);
            map.entry(basename.to_string()).or_default().insert(
                *locale,
                CachedBundle {
                    bundle: Arc::clone(&bundle),
                    loaded_at_millis: loaded_at,
                },
            );

            Arc::new(map)
        });

        Some(bundle)
    }

    fn get_cached_bundle(&self, basename: &str, locale: &Locale) -> Option<Arc<ResourceBundle>> {
        let cache = self.cached_resource_bundles.load();
        self.cached_bundle_from_map(&cache, basename, locale)
    }

    fn cached_bundle_from_map(
        &self,
        cache: &HashMap<String, HashMap<Locale, CachedBundle>>,
        basename: &str,
        locale: &Locale,
    ) -> Option<Arc<ResourceBundle>> {
        let cached = cache.get(basename)?.get(locale)?;

        if self.cache_millis >= 0
            && cached.loaded_at_millis <= current_time_millis().saturating_sub(self.cache_millis)
        {
            return None;
        }

        Some(Arc::clone(&cached.bundle))
    }

    fn load_bundle_hierarchy(
        &self,
        basename: &str,
        locale: &Locale,
    ) -> io::Result<HashMap<String, String>> {
        let mut merged = HashMap::new();

        for filename in self
            .calculate_bundle_filenames(basename, locale)
            .into_iter()
            .rev()
        {
            // Load from least specific to most specific so locale-specific entries can
            // override the generic defaults inside the same merged map.
            if let Some(properties) = self.load_bundle_properties(&filename)? {
                merged.extend(properties);
            }
        }

        if merged.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "No bundle found for basename {basename} and locale {}",
                    locale.as_str()
                ),
            ))
        } else {
            Ok(merged)
        }
    }

    fn calculate_bundle_filenames(&self, basename: &str, locale: &Locale) -> Vec<String> {
        // Preserve lookup precedence while avoiding duplicated filenames when the
        // request locale, default locale and system locale overlap.
        let mut filenames = self.calculate_locale_filenames(basename, locale);

        if let Some(default_locale) = self.default_locale {
            if default_locale != *locale {
                for filename in self.calculate_locale_filenames(basename, &default_locale) {
                    if !filenames.contains(&filename) {
                        filenames.push(filename);
                    }
                }
            }
        } else if self.fallback_to_system_locale {
            let system_locale = Locale::locale();
            if system_locale != *locale {
                for filename in self.calculate_locale_filenames(basename, &system_locale) {
                    if !filenames.contains(&filename) {
                        filenames.push(filename);
                    }
                }
            }
        }

        filenames.push(basename.to_string());
        filenames
    }

    fn calculate_locale_filenames(&self, basename: &str, locale: &Locale) -> Vec<String> {
        let normalized = locale.as_str().replace('-', "_");
        let mut parts = normalized
            .split('_')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();

        if parts.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(parts.len());
        while !parts.is_empty() {
            result.push(format!("{basename}_{}", parts.join("_")));
            parts.pop();
        }

        result
    }

    fn load_bundle_properties(
        &self,
        filename: &str,
    ) -> io::Result<Option<HashMap<String, String>>> {
        let resource_name = self.to_resource_name(filename, PROPERTIES_EXTENSION);
        let Some(data) = self.resource_loader.load(&resource_name) else {
            return Ok(None);
        };

        let source = decode_with_encoding(data.as_ref(), self.default_encoding.as_deref())?;
        Ok(Some(load_properties(&source)))
    }

    fn to_resource_name(&self, bundle_name: &str, extension: &str) -> String {
        let normalized_extension = extension.trim_start_matches('.');
        let path = bundle_name.replace('.', "/");
        format!("{MESSAGES}{path}.{normalized_extension}")
    }

    fn discover_bundle_locales(&self, basename: &str) -> HashSet<Option<Locale>> {
        let basename = format!("{}{}", MESSAGES, basename);
        let extension = format!(".{PROPERTIES_EXTENSION}");
        let mut locales = HashSet::new();

        for path in self.resource_loader.iter() {
            if !path.ends_with(&extension) || !path.starts_with(&basename) {
                continue;
            }

            let suffix = &path[basename.len()..path.len() - extension.len()];
            if suffix.is_empty() {
                locales.insert(None);
                continue;
            }

            let suffix = suffix
                .trim_start_matches('_')
                .trim_start_matches('.')
                .trim_start_matches('-');

            if suffix.is_empty() {
                locales.insert(None);
                continue;
            }

            if let Ok(locale) = suffix.replace('_', "-").parse::<Locale>() {
                locales.insert(Some(locale));
            }
        }

        locales
    }
}

impl std::fmt::Display for ResourceBundleMessageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ResourceBundleMessageSource: basenames={:?}",
            self.basenames
        )
    }
}

fn decode_with_encoding(data: &[u8], encoding: Option<&str>) -> io::Result<String> {
    match encoding {
        Some(label) => {
            let normalized = label.trim().to_ascii_lowercase();
            if normalized == "utf-8" || normalized == "utf8" {
                return String::from_utf8(data.to_vec())
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error));
            }

            if normalized == "iso-8859-1" || normalized == "latin1" {
                return Ok(data.iter().map(|&value| value as char).collect());
            }

            let Some(encoding) = encoding_from_whatwg_label(label) else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unsupported encoding: {label}"),
                ));
            };

            encoding
                .decode(data, encoding::DecoderTrap::Strict)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        }
        None => String::from_utf8(data.to_vec())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
    }
}

impl MessageSource for ResourceBundleMessageSource {
    fn message(&self, code: &str, locale: Locale) -> Option<String> {
        self.resolve_code_without_arguments(code, &locale)
    }

    fn message_with_args(&self, code: &str, args: &[&str], locale: Locale) -> Option<String> {
        self.resolve_code(code, &locale, args)
    }

    fn message_or_default(&self, code: &str, locale: Locale) -> String {
        self.message(code, locale).unwrap_or(code.to_string())
    }
}

impl Clone for ResourceBundleMessageSource {
    fn clone(&self) -> Self {
        Self {
            basenames: self.basenames.clone(),
            default_locale: self.default_locale.clone(),
            fallback_to_system_locale: self.fallback_to_system_locale,
            default_encoding: self.default_encoding.clone(),
            resource_loader: self.resource_loader.clone(),
            cache_millis: self.cache_millis,
            cached_resource_bundles: ArcSwap::new(self.cached_resource_bundles.load_full()),
        }
    }
}

fn load_properties(source: &str) -> HashMap<String, String> {
    let mut properties = HashMap::new();
    let mut current_line = String::new();

    for raw_line in source.lines() {
        let raw_line = raw_line.trim_end_matches('\r');
        let continuation = ends_with_unescaped_backslash(raw_line);
        let segment = if continuation {
            &raw_line[..raw_line.len().saturating_sub(1)]
        } else {
            raw_line
        };

        if current_line.is_empty() {
            current_line.push_str(segment);
        } else {
            // Continuation lines ignore leading whitespace as defined by .properties syntax.
            current_line.push_str(segment.trim_start());
        }

        if continuation {
            continue;
        }

        insert_property_line(&current_line, &mut properties);
        current_line.clear();
    }

    if !current_line.is_empty() {
        insert_property_line(&current_line, &mut properties);
    }

    properties
}

fn ends_with_unescaped_backslash(value: &str) -> bool {
    let mut count = 0;
    for byte in value.as_bytes().iter().rev() {
        if *byte == b'\\' {
            count += 1;
        } else {
            break;
        }
    }

    count % 2 == 1
}

fn insert_property_line(line: &str, properties: &mut HashMap<String, String>) {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
        return;
    }

    let (key, value) = split_property(trimmed);
    let key = unescape_property(key.trim());
    if key.is_empty() {
        return;
    }

    properties.insert(key, unescape_property(value.trim_start()));
}

fn split_property(line: &str) -> (&str, &str) {
    let mut escaped = false;

    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '=' | ':' => return (&line[..index], &line[index + ch.len_utf8()..]),
            ch if ch.is_whitespace() => {
                // Java .properties treats the first unescaped whitespace as a separator
                // unless it is escaped, and optional `=` / `:` may follow it.
                let mut value_start = index;

                while let Some(next) = line[value_start..].chars().next() {
                    if !next.is_whitespace() {
                        break;
                    }
                    value_start += next.len_utf8();
                }

                if let Some(next) = line[value_start..].chars().next() {
                    if next == '=' || next == ':' {
                        value_start += next.len_utf8();

                        while let Some(space) = line[value_start..].chars().next() {
                            if !space.is_whitespace() {
                                break;
                            }
                            value_start += space.len_utf8();
                        }
                    }
                }

                return (&line[..index], &line[value_start..]);
            }
            _ => {}
        }
    }

    (line, "")
}

fn unescape_property(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        match chars.next() {
            Some('t') => result.push('\t'),
            Some('r') => result.push('\r'),
            Some('n') => result.push('\n'),
            Some('f') => result.push('\u{000C}'),
            Some('\\') => result.push('\\'),
            Some(' ') => result.push(' '),
            Some(':') => result.push(':'),
            Some('=') => result.push('='),
            Some('#') => result.push('#'),
            Some('!') => result.push('!'),
            Some('u') => {
                // Avoid allocating an intermediate String for the common \uXXXX path.
                let remaining = chars.as_str().as_bytes();
                if remaining.len() >= 4
                    && remaining[..4].iter().all(|byte| byte.is_ascii_hexdigit())
                {
                    let digits = std::str::from_utf8(&remaining[..4]).unwrap_or_default();
                    if let Ok(code_point) = u32::from_str_radix(digits, 16) {
                        if let Some(decoded) = char::from_u32(code_point) {
                            result.push(decoded);
                            for _ in 0..4 {
                                chars.next();
                            }
                            continue;
                        }
                    }
                }

                result.push('\\');
                result.push('u');
            }
            Some(other) => result.push(other),
            None => result.push('\\'),
        }
    }

    result
}

fn parse_message_template(message: &str) -> MessageTemplate {
    let mut segments = Vec::new();
    let mut literal_start = 0;
    let bytes = message.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'{' {
            let placeholder_start = index;
            let mut cursor = index + 1;

            // Only cache the simple `{0}` / `{1}` style placeholders used by this
            // message source so rendering stays allocation-light on the hot path.
            if cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                    cursor += 1;
                }

                if cursor < bytes.len() && bytes[cursor] == b'}' {
                    if literal_start < placeholder_start {
                        segments.push(TemplateSegment::Literal {
                            start: literal_start,
                            end: placeholder_start,
                        });
                    }

                    let argument_index = message[placeholder_start + 1..cursor]
                        .parse::<usize>()
                        .unwrap_or_default();

                    segments.push(TemplateSegment::Argument {
                        index: argument_index,
                        start: placeholder_start,
                        end: cursor + 1,
                    });

                    index = cursor + 1;
                    literal_start = index;
                    continue;
                }
            }
        }

        let next_len = message[index..]
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(1);
        index += next_len;
    }

    if literal_start < message.len() {
        segments.push(TemplateSegment::Literal {
            start: literal_start,
            end: message.len(),
        });
    }

    if segments.is_empty() {
        segments.push(TemplateSegment::Literal {
            start: 0,
            end: message.len(),
        });
    }

    MessageTemplate {
        pattern: message.to_string(),
        segments,
    }
}

fn render_message_template(template: &MessageTemplate, args: &[impl AsRef<str>]) -> String {
    let mut capacity = template.pattern.len();
    for arg in args {
        capacity += arg.as_ref().len();
    }

    let mut rendered = String::with_capacity(capacity);

    for segment in &template.segments {
        match segment {
            TemplateSegment::Literal { start, end } => {
                rendered.push_str(&template.pattern[*start..*end]);
            }
            TemplateSegment::Argument { index, start, end } => {
                if let Some(arg) = args.get(*index) {
                    rendered.push_str(arg.as_ref());
                } else {
                    rendered.push_str(&template.pattern[*start..*end]);
                }
            }
        }
    }

    rendered
}

fn current_time_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_properties_handles_continuation_comments_and_unicode() {
        let source = "# comment\ngreeting = Hello\\\n    World\nescaped.key = value\\:\\=\\#\\!\nunicode = \\u4F60\\u597D\n";

        let properties = load_properties(source);

        assert_eq!(
            properties.get("greeting").map(String::as_str),
            Some("HelloWorld")
        );
        assert_eq!(
            properties.get("escaped.key").map(String::as_str),
            Some("value:=#!")
        );
        assert_eq!(properties.get("unicode").map(String::as_str), Some("你好"));
    }

    #[test]
    fn resource_bundle_parses_templates_lazily() {
        let bundle = ResourceBundle::new(
            "messages".to_string(),
            Locale::EnUs,
            HashMap::from([("welcome".to_string(), "Hello {0}".to_string())]),
        );

        assert!(
            bundle
                .templates
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_empty()
        );

        assert_eq!(
            bundle.format("welcome", &["Rust"]),
            Some("Hello Rust".to_string())
        );

        assert_eq!(
            bundle
                .templates
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .len(),
            1
        );
    }

    #[test]
    fn unescape_property_keeps_invalid_unicode_sequences_verbatim() {
        assert_eq!(unescape_property(r"\u12"), r"\u12");
        assert_eq!(unescape_property(r"\u12xz"), r"\u12xz");
    }
}
