use chrono::{DateTime, FixedOffset, Utc};

// =======================
// 1. 定义时间谓词枚举
// =======================
#[derive(Debug, Clone)]
pub enum ZonedDateTimeRoutePredicateFactory {
    After(Vec<DateTime<FixedOffset>>),
    Before(Vec<DateTime<FixedOffset>>),
    Between(Vec<(DateTime<FixedOffset>, DateTime<FixedOffset>)>),
}

impl ZonedDateTimeRoutePredicateFactory {
    pub fn matches(&self, _session: &mut pingora::protocols::http::ServerSession) -> bool {
        let now_utc = Utc::now();

        match self {
            Self::After(targets) => {
                targets.iter().any(|target| {
                    let target_utc = target.with_timezone(&Utc);
                    now_utc >= target_utc  // After: 当前时间 >= 目标时间
                })
            }

            Self::Before(targets) => {
                targets.iter().any(|target| {
                    let target_utc = target.with_timezone(&Utc);
                    now_utc < target_utc  // Before: 当前时间 < 目标时间
                })
            }

            Self::Between(ranges) => {
                ranges.iter().any(|(start, end)| {
                    let start_utc = start.with_timezone(&Utc);
                    let end_utc = end.with_timezone(&Utc);

                    // 确保 end > start（配置有效性）
                    if start_utc >= end_utc {
                        return false; // 无效区间，不匹配
                    }

                    now_utc >= start_utc && now_utc < end_utc // [start, end)
                })
            }
        }
    }
}

fn parse_zoned_datetime(s: &str) -> Result<DateTime<FixedOffset>, Box<dyn std::error::Error>> {
    // 示例输入: "2017-01-20T17:42:47.789-07:00[America/Denver]"
    // 提取时间+偏移部分: "2017-01-20T17:42:47.789-07:00"

    let clean: String = match s.find('[') {
        Some(pos) => s[..pos].to_string(),
        None => s.to_string(),
    };

    // 使用 chrono 解析 ISO 8601 带偏移格式
    DateTime::parse_from_rfc3339(&clean).map_err(|e| e.into())
}

pub(super) fn after(time_str: &str) -> Result<ZonedDateTimeRoutePredicateFactory, Box<dyn std::error::Error>> {
    let dt = parse_zoned_datetime(time_str)?;
    Ok(ZonedDateTimeRoutePredicateFactory::After(vec![dt]))
}

pub(super) fn before(time_str: &str) -> Result<ZonedDateTimeRoutePredicateFactory, Box<dyn std::error::Error>> {
    let dt = parse_zoned_datetime(time_str)?;
    Ok(ZonedDateTimeRoutePredicateFactory::Before(vec![dt]))
}

pub(super) fn between(
    start_str: &str,
    end_str: &str,
) -> Result<ZonedDateTimeRoutePredicateFactory, Box<dyn std::error::Error>> {
    let start = parse_zoned_datetime(start_str)?;
    let end = parse_zoned_datetime(end_str)?;

    if start >= end {
        return Err("Start time must be before end time".into());
    }

    Ok(ZonedDateTimeRoutePredicateFactory::Between(vec![(start, end)]))
}

