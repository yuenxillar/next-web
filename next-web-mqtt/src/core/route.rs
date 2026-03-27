use std::borrow::Cow;

use super::topic::base_topic::BaseTopic;

/// MQTT涓婚璺敱鍣?
/// 璐熻矗鏍规嵁涓婚鍖归厤瑙勫垯璺敱娑堟伅
///
/// MQTT Topic Router
/// Responsible for routing messages based on topic matching rules
pub struct TopicRoute {
    /// 涓婚
    ///
    /// Topic
    pub topic: Cow<'static, str>,
    /// 鍖归厤绫诲瀷
    ///
    /// Match type
    pub match_type: MacthType,
    /// 涓婚娑堣垂鑰?
    ///
    /// topic consumer
    pub base_topic: Box<dyn BaseTopic>,
}

/// 涓婚鍖归厤绫诲瀷
///
/// Topic match type
#[derive(Debug, Clone)]
pub enum MacthType {
    /// 澶氬眰閫氶厤绗﹀尮閰?璧峰绱㈠紩)
    ///
    /// Multi-level wildcard match (start index)
    Multilayer(usize),
    /// 鍗曞眰閫氶厤绗﹀尮閰?璧峰绱㈠紩,缁撴潫绱㈠紩)
    ///
    /// Single-level wildcard match (start index, end index)
    Singlelayer(usize, usize),
    /// 浠绘剰鍖归厤
    ///
    /// Match anything
    Anything,
}

impl TopicRoute {
    pub fn new<M: Into<Cow<'static, str>>>(
        topic: M,
        base_topic: Box<dyn BaseTopic>,
    ) -> Result<Self, String> {
        let topic = topic.into();

        let match_type = if topic.contains('#') {
            if topic.len() == 1 {
                MacthType::Anything
            } else if let Some(index) = topic.find('#') {
                MacthType::Multilayer(index)
            } else {
                return Err("invalid multi-level MQTT topic pattern".to_string());
            }
        } else if topic.contains('+') {
            let index: Vec<&str> = topic.split('+').collect();
            if index.len() == 2 {
                MacthType::Singlelayer(index[0].len(), index[1].len())
            } else {
                return Err(format!("unsupported single-level MQTT topic pattern: {topic}"));
            }
        } else {
            return Err(format!(
                "topic '{topic}' does not contain a supported wildcard pattern"
            ));
        };

        Ok(Self {
            topic,
            match_type,
            base_topic,
        })
    }
}
