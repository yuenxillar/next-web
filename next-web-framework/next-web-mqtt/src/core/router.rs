use std::borrow::Cow;

use super::topic::base_topic::BaseTopic;

pub struct TopicRouter {
    pub topic: Cow<'static, str>,
    pub match_type: MacthType,
    pub base_topic: Box<dyn BaseTopic>,
}

#[derive(Debug, Clone)]
pub enum MacthType {
    // index
    Multilayer(usize),
    Singlelayer(usize, usize),

    Anything,
}

impl TopicRouter {
    pub fn new<M: Into<Cow<'static, str>>>(topic: M, base_topic: Box<dyn BaseTopic>) -> Self {
        let topic = topic.into();

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
            let left = if let Some(index) = topic.find('+') {
                index
            } else {
                0
            };

            let mut right = left + 2;
            if right > topic.len() {
                right = 0;
            }
            MacthType::Singlelayer(left, right)
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
