/// 定义 `JsonObject` 操作中可能出现的错误类型。
///
/// Defines error types that may occur during `JsonObject` operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonObjectError {
    /// 键为空。
    ///
    /// Key is empty.
    KeyIsEmpty,
    /// 键已存在。
    ///
    /// Key already exists.
    KeyAlreadyExists,
    /// 值为 `null`。
    ///
    /// Value is null.
    ValueIsNull,
    /// 键或值为 `null`。
    ///
    /// Key or value is null.
    KeyOrValueIsNull,
    /// 解析错误。
    ///
    /// Parse error.
    ParseError,
}

impl std::fmt::Display for JsonObjectError {
    /// 格式化错误信息。
    ///
    /// Formats the error message.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonObjectError::KeyIsEmpty => write!(f, "键为空"),
            JsonObjectError::KeyAlreadyExists => write!(f, "键已存在"),
            JsonObjectError::ValueIsNull => write!(f, "值为 null"),
            JsonObjectError::KeyOrValueIsNull => write!(f, "键或值为 null"),
            JsonObjectError::ParseError => write!(f, "解析错误"),
        }
    }
}

impl std::error::Error for JsonObjectError {}