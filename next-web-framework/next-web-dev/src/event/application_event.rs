use crate::util::date_time_util::LocalDateTimeUtil;



pub trait ApplicationEvent: Send + Sync + 'static {
    fn get_timestamp(&self) -> i64 {
        LocalDateTimeUtil::timestamp()
    }
}