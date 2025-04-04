use std::{any::TypeId, borrow::Cow, fmt::{self, Display}};

/// 事件键
/// Event key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    id: Cow<'static, str>,
    tid: TypeId,
}

impl Key {
    /// 创建新的事件键
    /// Create a new event key
    pub fn new(id: impl Into<Cow<'static, str>>, tid: TypeId) -> Self {
        Self {
            id: id.into(),
            tid,
        }
    }

    /// 获取事件键的ID
    /// Get the ID of the event key
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 获取事件键的类型ID
    /// Get the type ID of the event key
    pub fn tid(&self) -> TypeId {
        self.tid
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key {{ id: {}, tid: {:?} }}", self.id, self.tid)
    }
}