use std::any::{Any, TypeId};

use crate::util::local_date_time::LocalDateTime;

/// 应用事件    
/// Application event
pub trait ApplicationEvent: Any + Send + Sync + 'static {
    /// 获取事件时间戳
    /// Get event timestamp
    fn timestamp(&self) -> i64 {
        LocalDateTime::timestamp()
    }

    /// 获取事件源
    fn source(&self) -> String {
        std::any::type_name::<Self>()
            .to_string()
            .rsplit("::")
            .next()
            .unwrap_or_default()
            .into()
    }

    /// 获取事件类型ID
    /// Get event type ID
    fn event_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}
