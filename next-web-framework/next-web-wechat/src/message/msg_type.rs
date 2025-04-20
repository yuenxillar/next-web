use serde::{Deserialize, Serialize};


#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum MsgType {
    Text,
    Image,
    Voice,
    Video,
    Music,
    ShortVideo,
    Location,
    // 图文
    News,
    Link,
    Event,
    
    // 更多....
}