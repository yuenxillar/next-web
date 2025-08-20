use chrono::DateTime;
use chrono_tz::Tz;
use super::route_predicate::RoutePredicate;

/// A factory that creates a route predicate that checks if the current time is within a certain time zone.
/// 
/// This predicate checks if the current time falls within the time window defined by
/// each configured `DateTime<Tz>` ± `offset` minutes.
#[derive(Debug, Clone)]
pub struct ZonedDateTimeRoutePredicateFactory {
    pub datetime: Vec<DateTime<Tz>>,
    pub offset: u8,
}

impl RoutePredicate for ZonedDateTimeRoutePredicateFactory {
    fn matches(&self, _session: &mut pingora::protocols::http::ServerSession) -> bool {
        // 1. 获取当前系统时间（UTC）
        let now_utc = chrono::Utc::now();
        
        // 2. 遍历每一个配置的时间点
        self.datetime.iter().any(|target_datetime| {
            // target_datetime 是一个带有时区的 DateTime
            // 我们需要将 'now_utc' 转换到 target_datetime 的时区进行比较
            // 或者将 target_datetime 转换到 UTC。这里选择后者以避免时区转换的复杂性。
            
            // 将目标时间转换为 UTC 进行比较
            let target_utc = target_datetime.with_timezone(&chrono::Utc);
            
            // 计算时间窗口的边界
            let window_start = target_utc 
                - chrono::Duration::minutes(i64::from(self.offset));
            let window_end = target_utc 
                + chrono::Duration::minutes(i64::from(self.offset));
            
            // 检查当前时间是否在窗口内 [start, end)
            // 使用左闭右开区间 [start, end) 是常见做法，避免边界重复
            now_utc >= window_start && now_utc < window_end
        })
    }
}