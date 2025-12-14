use crate::wechat_basic_message;

//use serde::Deserialize;
// use std::str::FromStr;

// #[derive(Debug, Deserialize)]
// #[serde(rename_all = "lowercase")]
// enum MessageBody {
//     Text(TextMessage),
//     Image(ImageMessage),
//     Voice(VoiceMessage),
//     Video(VideoMessage),
//     Music(MusicMessage),
//     ShortVideo(ShortVideoMessage),
//     Location(LocationMessage),
//     News(NewsMessage),
//     Link(LinkMessage),
//     Event(EventMessage),
//     #[serde(other)]
//     Unknown,
// }

wechat_basic_message!(TextMessage {
    ///文本内容
    content: String,
    /// 消息id，64位整型
    msg_id: u64,
    /// 消息的数据ID（消息如果来自文章时才有）
    msg_data_id: Option<u64>,
    /// 多图文时第几篇文章，从1开始（消息如果来自文章时才有）
    idx: Option<u32>,
});

wechat_basic_message!(ImageMessage {
    /// 图片链接（由系统生成）
    pic_url: String,
    /// 图片消息媒体id，可以调用获取临时素材接口拉取数据。
    media_id: String,
    /// 消息id，64位整型
    msg_id: u64,
    /// 消息的数据ID（消息如果来自文章时才有）
    msg_data_id: Option<u64>,
    /// 多图文时第几篇文章，从1开始（消息如果来自文章时才有）
    idx: Option<u32>,
});

wechat_basic_message!(VoiceMessage {
    /// 语音消息媒体id，可以调用获取临时素材接口拉取数据，Format为amr时返回8K采样率amr语音。
    media_id: String,
    /// 语音格式，如amr，speex等
    format: String,
    /// 消息id，64位整型
    msg_id: u64,
    /// 消息的数据ID（消息如果来自文章时才有）
    msg_data_id: Option<u64>,
    /// 多图文时第几篇文章，从1开始（消息如果来自文章时才有）
    idx: Option<u32>,
    /// 16K采样率语音消息媒体id，可以调用获取临时素材接口拉取数据，返回16K采样率amr/speex语音。
    media_id16_k: Vec<u8>,

});

wechat_basic_message!(VideoMessage {
    /// 视频消息媒体id，可以调用获取临时素材接口拉取数据
    media_id: String,
    /// 视频消息缩略图的媒体id，可以调用多媒体文件下载接口拉取数据
    thumb_media_id: String,
    /// 消息id，64位整型
    msg_id: u64,
    /// 消息的数据ID（消息如果来自文章时才有）
    msg_data_id: Option<u64>,
    /// 多图文时第几篇文章，从1开始（消息如果来自文章时才有）
    idx: Option<u32>,
});

wechat_basic_message!(ShortVideoMessage {
    /// 视频消息媒体id，可以调用获取临时素材接口拉取数据。
    media_id: String,
    /// 视频消息缩略图的媒体id，可以调用获取临时素材接口拉取数据。
    thumb_media_id: String,
    /// 消息id，64位整型
    msg_id: u64,
    /// 消息的数据ID（消息如果来自文章时才有）
    msg_data_id: Option<u64>,
    /// 多图文时第几篇文章，从1开始（消息如果来自文章时才有）
    idx: Option<u32>,
});

wechat_basic_message!(LocationMessage {
    /// 地理位置纬度
    #[serde(rename = "Location_X")]
    location_x: f64,
    /// 地理位置经度
    #[serde(rename = "Location_Y")]
    location_y: f64,
    ///	地图缩放大小
    scale: i32,
    /// 地理位置信息
    label: String,
    /// 消息id，64位整型
    msg_id: u64,
    /// 消息的数据ID（消息如果来自文章时才有）
    msg_data_id: Option<u64>,
    /// 多图文时第几篇文章，从1开始（消息如果来自文章时才有）
    idx: Option<u32>,
});

wechat_basic_message!(LinkMessage {
    /// 消息标题
    title: String,
    /// 消息描述
    description: String,
    /// 消息链接
    url: String,
    /// 消息id，64位整型
    msg_id: u64,
    /// 消息的数据ID（消息如果来自文章时才有）
    msg_data_id: Option<u64>,
    /// 多图文时第几篇文章，从1开始（消息如果来自文章时才有）
    idx: Option<u32>,
});

////////////////////////////////////////
wechat_basic_message!(NewsMessage {
    /// 图文消息个数；当用户发送文本、图片、语音、视频、图文、地理位置这六种消息时，开发者只能回复1条图文消息；其余场景最多可回复8条图文消息
    article_count: u32,
    /// 图文消息信息，注意，如果图文数超过限制，则将只发限制内的条数
    articles: String,
    /// 图文消息标题
    title: String,
    /// 图文消息描述
    description: String,
    /// 图片链接，支持JPG、PNG格式，较好的效果为大图360*200，小图200*200
    pic_url: String,
    /// 点击图文消息跳转链接
    url: String,
});

wechat_basic_message!(MusicMessage {
        /// 音乐标题
        title: Option<String>,
        /// 音乐描述
        description: Option<String>,
        /// 音乐链接
        music_url: Option<String>,
        /// 高质量音乐链接，WIFI环境优先使用该链接播放音乐
        hq_music_url: Option<String>,
        /// 缩略图的媒体id，通过素材管理中的接口上传多媒体文件，得到的id
        thumb_media_id: String,
    }
);

// Event Message

// https://developers.weixin.qq.com/doc/offiaccount/Message_Management/Receiving_event_pushes.html

// 关注/取消关注事件
wechat_basic_message!(FollowEventMessage {
    /// 事件类型，subscribe(订阅)、unsubscribe(取消订阅)
    event: String,
});

// 扫描带参数二维码事件
wechat_basic_message!(ScanQRCodeEventMessage {
    /// 事件类型，SCAN
    event: String,
    /// 事件KEY值，为二维码的场景值ID
    event_key: String,
    /// 二维码的ticket，可用来换取二维码图片
    ticket: String
});

// 用户已关注时的事件推送
wechat_basic_message!(UserFollowedEventMessage {
    /// 事件类型，SCAN
    event: String,
    /// 事件KEY值，为二维码的场景值ID
    event_key: String,
    /// 二维码的ticket，可用来换取二维码图片
    ticket: String
});

// 上报地理位置事件
wechat_basic_message!(ReportLocationEventMessage {
    /// 事件类型 LOCATION
    event: String,
    /// 地理位置纬度
    latitude: f64,
    /// 地理位置经度
    longitude: f64,
    /// 地理位置精度
    precision: f64
});

// 自定义菜单事件
wechat_basic_message!(CustomizeMenuEventMessage {
    /// 事件类型 CLICK
    event: String,
    /// 事件KEY值，与自定义菜单接口中KEY值对应
    event_key: String,
});

// impl Default for MessageBody {
//     fn default() -> Self {
//         MessageBody::Unknown
//     }
// }
