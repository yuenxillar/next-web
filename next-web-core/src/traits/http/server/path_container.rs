use std::fmt;
use std::sync::Arc;

// ============================================================
// MultiValueMap 占位
// ============================================================
pub type MultiValueMap<K, V> = std::collections::HashMap<K, Vec<V>>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Options {
    separator: char,
    decode_and_parse_segments: bool,
}

impl Options {
    pub const HTTP_PATH: Options = Options::new('/', true);
    pub const MESSAGE_ROUTE: Options = Options::new('.', false);

    const fn new(separator: char, decode_and_parse_segments: bool) -> Self {
        Self {
            separator,
            decode_and_parse_segments,
        }
    }

    pub fn create(separator: char, decode_and_parse_segments: bool) -> Self {
        Self::new(separator, decode_and_parse_segments)
    }

    pub fn separator(&self) -> char {
        self.separator
    }

    pub fn should_decode_and_parse_segments(&self) -> bool {
        self.decode_and_parse_segments
    }
}

impl Default for Options {
    fn default() -> Self {
        Options::HTTP_PATH
    }
}

// ============================================================
// Element Trait
// ============================================================

pub trait Element: fmt::Debug + Send + Sync {
    fn value(&self) -> &str;
}

// ============================================================
// PathSegment Trait
// ============================================================

pub trait PathSegment: Element {
    fn value_to_match(&self) -> &str;

    fn value_to_match_as_chars(&self) -> Vec<char> {
        self.value_to_match().chars().collect()
    }

    fn parameters(&self) -> MultiValueMap<String, String>;
}

// ============================================================
// Separator Trait
// ============================================================

pub trait Separator: Element {}

// ============================================================
// PathContainer Trait
// ============================================================

pub trait PathContainer {
    fn value(&self) -> &str;

    fn elements(&self) -> &[Box<dyn Element>];

    fn sub_path(&self, start_index: usize, end_index: usize) -> Arc<dyn PathContainer> {
        DefaultPathContainer::sub_path(self, start_index, end_index)
    }

    fn sub_path_from(&self, index: usize) -> Arc<dyn PathContainer> {
        self.sub_path(index, self.elements().len())
    }

    fn parse_path(path: &str) -> Arc<dyn PathContainer> {
        DefaultPathContainer::create_from_url_path(path, Options::HTTP_PATH)
    }

    fn parse_path_with_options(path: &str, options: Options) -> Arc<dyn PathContainer> {
        DefaultPathContainer::create_from_url_path(path, options)
    }
}

// ============================================================
// DefaultPathContainer 占位实现
// ============================================================

pub struct DefaultPathContainer {
    value: String,
    elements: Vec<Box<dyn Element>>,
    options: Options,
}

impl DefaultPathContainer {
    pub fn create_from_url_path(path: &str, options: Options) -> Arc<dyn PathContainer> {
        // 实际实现需要：
        // 1. 根据 options.separator 切分路径
        // 2. 根据 options.decode_and_parse_segments 决定是否解码和解析参数
        // 3. 构建 Element 列表（Separator 和 PathSegment）
        todo!(
            "Implement URL path parsing for '{}' with separator '{}'",
            path,
            options.separator()
        )
    }

    pub fn sub_path(
        container: &dyn PathContainer,
        start_index: usize,
        end_index: usize,
    ) -> Arc<dyn PathContainer> {
        let elements = container.elements();
        let end_index = end_index.min(elements.len());
        let start_index = start_index.min(end_index);

        // 实际实现需要从原始容器中提取子路径
        todo!("Implement sub_path from {} to {}", start_index, end_index)
    }
}

impl fmt::Debug for DefaultPathContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultPathContainer")
            .field("value", &self.value)
            .field("elements_count", &self.elements.len())
            .field("options", &self.options)
            .finish()
    }
}

impl PathContainer for DefaultPathContainer {
    fn value(&self) -> &str {
        &self.value
    }

    fn elements(&self) -> &[Box<dyn Element>] {
        &self.elements
    }
}
