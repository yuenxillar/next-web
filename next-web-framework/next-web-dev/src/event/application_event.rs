use std::any::Any;
use std::any::TypeId;

use crate::util::date_time_util::LocalDateTimeUtil;


/// 应用事件    
/// Application event
pub trait ApplicationEvent: Send + Sync + 'static {

    /// 获取事件时间戳
    /// Get event timestamp
    fn get_timestamp(&self) -> i64 {
        LocalDateTimeUtil::timestamp()
    }

     /// 获取事件源
     fn source(&self) -> Option<&dyn Any>;


     /// 获取事件类型ID
     /// Get event type ID
     fn tid(&self) -> TypeId {
        TypeId::of::<Self>()
     }
}