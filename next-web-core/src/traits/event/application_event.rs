use std::any::{Any, TypeId};

use dyn_clone::DynClone;

pub type EventId = (String, TypeId);

/// 应用事件
///
/// Application event
pub trait ApplicationEvent
where
    Self: Send + Sync,
    Self: Any + DynClone,
{
    /// 获取事件时间戳
    ///
    /// Get event timestamp
    fn timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// 获取事件源
    ///
    /// Get event source
    fn source(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// 获取事件类型ID
    ///
    /// Get event type ID
    fn event_id(&self) -> EventId {
        (self.id(), TypeId::of::<Self>())
    }

    fn id(&self) -> String;
}

dyn_clone::clone_trait_object!(ApplicationEvent);
