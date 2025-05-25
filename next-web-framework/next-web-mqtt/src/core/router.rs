use std::borrow::Cow;

use super::topic::base_topic::BaseTopic;

/// MQTT主题路由器
/// MQTT Topic Router
///
/// 负责根据主题匹配规则路由消息
/// Responsible for routing messages based on topic matching rules
pub struct TopicRouter {
    /// 主题字符串
    /// Topic string
    pub topic: Cow<'static, str>,
    /// 匹配类型
    /// Match type
    pub match_type: MacthType,
    /// 基础主题处理器
    /// Base topic handler
    pub base_topic: Box<dyn BaseTopic>,
}

/// 主题匹配类型
/// Topic match type
#[derive(Debug, Clone)]
pub enum MacthType {
    /// 多层通配符匹配(起始索引)
    /// Multi-level wildcard match (start index)
    Multilayer(usize),
    /// 单层通配符匹配(起始索引,结束索引)
    ///  Single-level wildcard match (start index, end index)
    Singlelayer(usize, usize),
    /// 任意匹配
    ///  Match anything
    Anything,
}

impl TopicRouter {
    /// 创建新的主题路由器
    ///  Create new topic router
    ///
    /// # 参数 / Parameters
    /// - `topic`: 主题字符串 / Topic string
    /// - `base_topic`: 基础主题处理器 / Base topic handler
    ///
    /// # 返回值 / Returns
    /// 返回新的TopicRouter实例 / Returns new TopicRouter instance
    pub fn new<M: Into<Cow<'static, str>>>(topic: M, base_topic: Box<dyn BaseTopic>) -> Self {
        let topic = topic.into();

        // 根据主题中的通配符确定匹配类型 
        // Determine match type based on wildcards in topic
        let match_type = if topic.contains("#") {
            if topic.len() == 1 {
                MacthType::Anything
            } else {
                MacthType::Multilayer(if let Some(index) = topic.find('#') {
                    index
                } else {
                    0
                })
            }
        } else if topic.contains("+") {
            let index: Vec<&str> = topic.split("+").collect();
            if index.len() == 2 {
                MacthType::Singlelayer(index[0].len(), index[1].len())
            }else {
                MacthType::Singlelayer(0, 0)
            }
        } else {
            panic!("This topic doesn't seem to have the desired matching type.");
        };

        Self {
            topic,
            match_type,
            base_topic,
        }
    }
}
