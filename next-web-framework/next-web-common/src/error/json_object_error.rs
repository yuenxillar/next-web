
/// 定义 `JsonObject` 操作中可能出现的错误类型。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonObjectError {
    /// 键为空。
    KeyIsEmpty,
    /// 键已存在。
    KeyAlreadyExists,
    /// 值为 `null`。
    ValueIsNull,
    /// 键或值为 `null`。
    KeyOrValueIsNull,
    /// 解析错误。
    PaseError,
}

impl std::fmt::Display for JsonObjectError {
    /// 格式化错误信息。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonObjectError::KeyIsEmpty => write!(f, "Key is empty"),
            JsonObjectError::KeyAlreadyExists => write!(f, "Key already exists"),
            JsonObjectError::ValueIsNull => write!(f, "Value is null"),
            JsonObjectError::KeyOrValueIsNull => write!(f, "Key or value is null"),
            JsonObjectError::PaseError => write!(f, "Parse error"),
        }
    }
}

impl std::error::Error for JsonObjectError {}
