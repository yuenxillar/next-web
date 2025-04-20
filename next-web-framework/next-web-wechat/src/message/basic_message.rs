use std::error::Error;
use std::fmt;

// 定义一个宏，用于为结构体添加默认的微信消息基本字段
#[macro_export]
macro_rules! wechat_basic_message {
    // 混合属性和非属性字段的通用模式
    ($name:ident {
        $($(#[$attr:meta])* $field_name:ident: $field_type:ty),* $(,)?
    }) => {
        #[derive(Debug, ::serde::Deserialize, Clone, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        pub struct $name {
            /// 开发者微信号
            pub to_user_name: String,
            /// 发送方账号（一个OpenID）
            pub from_user_name: String,
            /// 消息创建时间 （整型）
            pub create_time: u64,
            /// 消息类型
            pub msg_type: $crate::message::msg_type::MsgType,
            $(
                $(#[$attr])*
                pub $field_name: $field_type,
            )*
        }
    };
}

#[derive(Debug)]
pub struct MessageParseError {
    pub message: String,
}

impl fmt::Display for MessageParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Message Parse Error: {}", self.message)
    }
}

impl Error for MessageParseError {}

#[cfg(test)]
mod tests {

    use crate::message::message_body::LocationMessage;

    #[test]
    fn test_macro_generated_struct() {
        let xml_str = "
        <xml>
  <ToUserName><![CDATA[toUser]]></ToUserName>
  <FromUserName><![CDATA[fromUser]]></FromUserName>
  <CreateTime>1351776360</CreateTime>
  <MsgType><![CDATA[location]]></MsgType>
  <Location_X>23.134521</Location_X>
  <Location_Y>113.358803</Location_Y>
  <Scale>20</Scale>
  <Label>TEST LABEL</Label>
  <MsgId>1234567890123456</MsgId>
  <MsgDataId>101</MsgDataId>
  <Idx>10</Idx>
</xml>";
        // 测试宏生成的结构体
        let result: LocationMessage = quick_xml::de::from_str(xml_str).unwrap();
        println!("result: {:?}", result);
    }
}
