use futures_util::future::BoxFuture;

pub trait AlipayMsgHandler
where
    Self: Send + Sync,
    Self: 'static,
{
    /// 客户端接收到消息后回调此方法
    /// 接收到的消息的消息api名
    /// msgId 接收到的消息的消息id
    /// bizContent 接收到的消息的内容，json格式
    ///
    fn on_message<'a>(
        &'a self,
        msg_api: String,
        msg_id: String,
        biz_content: String,
    ) -> BoxFuture<'a, ()>;
}
