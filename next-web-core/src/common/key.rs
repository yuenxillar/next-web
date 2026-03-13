use std::{
    any::TypeId,
    borrow::Cow,
    fmt::{self, Display},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    id: Cow<'static, str>,
    tid: TypeId,
}

impl Key {
    /// Create a new event key
    ///
    /// 创建新的事件键
    pub fn new(id: impl Into<Cow<'static, str>>, tid: TypeId) -> Self {
        Self { id: id.into(), tid }
    }

    /// Get the ID of the event key
    ///
    /// 获取事件键的ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the type ID of the event key
    ///
    /// 获取事件键的类型ID
    pub fn type_id(&self) -> TypeId {
        self.tid
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key {{ id: {}, tid: {:?} }}", self.id, self.tid)
    }
}
