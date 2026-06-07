use crate::http::server::path_container::Separator;
use crate::http::server::{PathOptions, PathSegment};

use std::collections::BTreeMap;
use std::fmt;
use std::sync::LazyLock;
use std::{collections::HashMap, sync::Arc};

use super::path_container::{Element, MultiValueMap, PathContainer};

/// 空参数常量（对应 Java 的 EMPTY_PARAMS）
const EMPTY_PARAMS: LazyLock<MultiValueMap> = LazyLock::new(BTreeMap::new);

/// 全局分隔符注册表
static SEPARATORS: LazyLock<HashMap<char, DefaultSeparator>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(
        '/',
        DefaultSeparator {
            separator: "/",
            encoded_sequence: "%2F",
        },
    );
    map.insert(
        '.',
        DefaultSeparator {
            separator: ".",
            encoded_sequence: "%2E",
        },
    );
    map
});

/// PathContainer 的默认实现
///
/// 将 URL 路径字符串解析为结构化的 Element 列表
///
/// # 解析流程
///
/// ```text
/// 输入: "/users;status=active/42/posts?q=test"
///                    ↓
/// 按分隔符 '/' 拆分，保留分隔符
///                    ↓
/// [Separator, "users;status=active", Separator, "42", Separator, "posts"]
///                    ↓
/// 对每个段解码 URI + 解析矩阵参数
///                    ↓
/// [Sep, Segment("users" params:{status:[active]}), Sep, Segment("42"), Sep, Segment("posts")]
/// ```
///
#[derive(Clone)]
pub struct DefaultPathContainer {
    path: String,
    elements: Vec<Arc<dyn Element>>,
}

/// 空路径单例
static EMPTY_PATH: LazyLock<Arc<dyn PathContainer>> = LazyLock::new(|| {
    Arc::new(DefaultPathContainer {
        path: String::new(),
        elements: Vec::new(),
    }) as Arc<dyn PathContainer>
});

impl DefaultPathContainer {
    // pub fn sub_path(path: &str, from_index: usize, to_index: usize) -> &str {}

    // pub fn create_from_url_path(path: &str, options: PathOptions) -> &str {}

    pub fn new(path: impl Into<String>, elements: Vec<Arc<dyn Element>>) -> Self {
        Self {
            path: path.into(),
            elements,
        }
    }

    pub fn value(&self) -> &str {
        &self.path
    }

    pub fn elements(&self) -> &[Arc<dyn Element>] {
        &self.elements
    }

    /// 从 URL 路径字符串创建 PathContainer
    pub fn create_from_url_path(path: &str, options: &PathOptions) -> Arc<dyn PathContainer> {
        if path.is_empty() {
            return EMPTY_PATH.clone();
        }

        let separator = options.separator();
        let default_separator = match SEPARATORS.get(&separator) {
            Some(separator) => separator,
            None => panic!("Unexpected separator: '{}'", separator),
        };

        let mut elements: Vec<Arc<dyn Element>> = Vec::new();
        let mut begin = 0;

        let cl_default_separator = Arc::new(default_separator.clone());
        if path.starts_with(separator) {
            begin = separator.len_utf8();
            elements.push(cl_default_separator.clone());
        }

        while begin < path.len() {
            let end = path[begin..].find(separator).map(|pos| begin + pos);

            let segment = match end {
                Some(e) => &path[begin..e],
                None => &path[begin..],
            };

            if !segment.is_empty() {
                let element = if options.should_decode_and_parse_segments() {
                    Self::decode_and_parse_path_segment(segment)
                } else {
                    DefaultPathSegment::from_separator(segment, default_separator)
                };
                elements.push(Arc::new(element));
            }

            let e = match end {
                Some(e) => e,
                None => break,
            };

            elements.push(cl_default_separator.clone());
            begin = e + separator.len_utf8();
        }

        Arc::new(DefaultPathContainer {
            path: path.to_string(),
            elements,
        })
    }

    /// 解码并解析路径段：URI 解码 + 提取矩阵参数
    ///
    ///
    /// # 示例
    /// ```text
    /// 输入: "users;status=active,verified;role=admin"
    /// 输出: PathSegment {
    ///     value: "users;status=active,verified;role=admin",
    ///     value_to_match: "users",
    ///     parameters: {status: ["active", "verified"], role: ["admin"]}
    /// }
    /// ```
    fn decode_and_parse_path_segment(segment: &str) -> DefaultPathSegment {
        match segment.find(';') {
            Some(index) => {
                // 有矩阵参数
                let raw_value = &segment[..index];
                let value_to_match = uri_decode(raw_value);
                let params_content = &segment[index..]; // 包含开头的 ';'
                let parameters = Self::parse_path_params(params_content);
                DefaultPathSegment::new(segment, &value_to_match, parameters)
            }
            None => {
                // 无矩阵参数，只做 URI 解码
                let value_to_match = uri_decode(segment);
                DefaultPathSegment::from_value_and_match(segment, &value_to_match)
            }
        }
    }

    /// 解析矩阵参数字符串
    ///
    /// 输入: ";status=active,verified;role=admin;flag"
    fn parse_path_params(input: &str) -> MultiValueMap {
        let mut result: MultiValueMap = BTreeMap::new();
        let mut begin = 1; // 跳过开头的 ';'

        while begin < input.len() {
            // 查找下一个 ';'
            let end = input[begin..].find(';').map(|pos| begin + pos);

            let param = match end {
                Some(e) => &input[begin..e],
                None => &input[begin..],
            };

            // 解析单个参数（可能含逗号分隔的多值）
            Self::parse_path_param_values(param, &mut result);

            match end {
                Some(e) => begin = e + 1,
                None => break,
            }
        }

        result
    }

    /// 解析单个矩阵参数的值（支持逗号分隔的多值）
    ///
    /// # 示例
    /// ```text
    /// "status=active,verified" → {status: [active, verified]}
    /// "flag"                   → {flag: [""]}
    /// ```
    fn parse_path_param_values(input: &str, output: &mut MultiValueMap) {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return;
        }

        match trimmed.find('=') {
            Some(index) => {
                // key=value[,value2,...]
                let name = uri_decode(trimmed[..index].trim());
                if name.is_empty() {
                    return;
                }

                let values_str = &trimmed[index + 1..];
                // 逗号分隔的多值： "active,verified" → ["active", "verified"]
                for v in values_str.split(',') {
                    let v = v.trim();
                    if v.is_empty() {
                        continue; // 跳过空值
                    }

                    let decoded = uri_decode(v);
                    output.entry(name.clone()).or_default().push(decoded);
                }
            }
            None => {
                // 无值参数：只放 key
                let name = uri_decode(trimmed);
                if !name.is_empty() {
                    output
                        .entry(trimmed.to_string())
                        .or_default()
                        .push(String::new());
                }
            }
        }
    }

    /// 创建子路径
    pub fn sub_path(
        container: &Arc<dyn PathContainer>,
        from_index: usize,
        to_index: usize,
    ) -> Arc<dyn PathContainer> {
        let elements = container.elements();

        // 快速路径：全范围
        if from_index == 0 && to_index == elements.len() {
            return container.clone();
        }

        // 空子路径
        if from_index == to_index {
            return EMPTY_PATH.clone();
        }

        // 边界检查
        assert!(
            from_index < elements.len(),
            "Invalid from_index: {} (size: {})",
            from_index,
            elements.len(),
        );
        assert!(
            to_index <= elements.len(),
            "Invalid to_index: {} (size: {})",
            to_index,
            elements.len(),
        );
        assert!(
            from_index < to_index,
            "from_index ({}) should be < to_index ({})",
            from_index,
            to_index,
        );

        let sub_elements = elements[from_index..to_index].to_vec();
        let path: String = sub_elements.iter().map(|e| e.value()).collect();

        Arc::new(DefaultPathContainer {
            path,
            elements: sub_elements,
        })
    }
}

impl PathContainer for DefaultPathContainer {
    fn value(&self) -> &str {
        &self.path
    }

    fn elements(&self) -> &[Arc<dyn Element>] {
        &self.elements
    }
}

impl PartialEq for DefaultPathContainer {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for DefaultPathContainer {}

impl fmt::Display for DefaultPathContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

/// 路径分隔符的编码序列
///
/// '/' 在路径段中编码为 `%2F`，'.' 编码为 `%2E`
/// 当段中包含这些编码序列时，需要解码回原始字符才能用于匹配
#[derive(Debug, Clone)]
pub struct DefaultSeparator {
    pub separator: &'static str,
    pub encoded_sequence: &'static str,
}

impl DefaultSeparator {
    pub fn encoded_sequence(&self) -> &str {
        self.encoded_sequence
    }
}

impl Separator for DefaultSeparator {}

impl Element for DefaultSeparator {
    fn value(&self) -> &str {
        self.separator
    }
}

/// 路径段的具体实现
///
/// 包含三个关键值：
/// - `value`：原始 URL 片段（可能包含编码序列和矩阵参数）
/// - `value_to_match`：解码后的匹配值（编码序列被还原，矩阵参数被剥离）
/// - `parameters`：矩阵参数（`;key=value` 形式）
#[derive(Debug, Clone, PartialEq)]
pub struct DefaultPathSegment {
    value: String,
    value_to_match: String,
    parameters: MultiValueMap,
}

impl DefaultPathSegment {
    pub fn new(value: &str, value_to_match: &str, parameters: MultiValueMap) -> Self {
        Self {
            value: value.to_string(),
            value_to_match: value_to_match.to_string(),
            parameters,
        }
    }

    /// 从原始值创建，处理编码序列（不解码 URI，只替换 %2F → / 等）
    pub fn from_separator(value: &str, separator: &DefaultSeparator) -> Self {
        let value_to_match = if value.contains(separator.encoded_sequence()) {
            value.replace(separator.encoded_sequence(), separator.value())
        } else {
            value.to_string()
        };
        Self::from_value_and_match(value, &value_to_match)
    }

    /// 从原始值和匹配值创建（无参数）
    pub fn from_value_and_match(value: &str, value_to_match: &str) -> Self {
        Self {
            value: value.to_string(),
            value_to_match: value_to_match.to_string(),
            parameters: EMPTY_PARAMS.clone(),
        }
    }
}

impl PathSegment for DefaultPathSegment {
    fn value_to_match(&self) -> &str {
        &self.value_to_match
    }

    fn value_to_match_as_chars(&self) -> Vec<char> {
        self.value_to_match.chars().collect::<Vec<_>>()
    }

    fn parameters(&self) -> &MultiValueMap {
        &self.parameters
    }
}

impl Element for DefaultPathSegment {
    fn value(&self) -> &str {
        &self.value
    }
}

/// URI 百分号解码
///
/// `%20` → ` `, `%2F` → `/`, `%E4%B8%AD` → `中`
fn uri_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                result.push(high * 16 + low);
                i += 3;
                continue;
            }
        }
        // 路径段解码：+ 保持原样，不转为空格
        result.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&result).into_owned()
}

/// 单个十六进制字符转数值
fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
