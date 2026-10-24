use futures_util::future::BoxFuture;

/// 支付宝消息处理器
///
/// 实现此 trait 即可自定义处理支付宝推送的各种消息事件。
/// 当支付宝服务器向您的回调地址推送消息时，会调用实现类的 `on_message` 方法。
///
pub trait AlipayMsgHandler
where
    Self: Send + Sync,
    Self: 'static,
{
    /// 处理支付宝推送的消息
    /// 当支付宝服务器向您发送消息时，此方法会被调用。
    ///
    /// # 参数
    ///
    /// * `msg_api` - 消息的 API 名称（即消息类型），用于区分不同业务场景
    ///   - 对应支付宝回调参数中的 `msg_api` 字段
    ///
    /// * `msg_id` - 消息的唯一标识 ID
    ///   - 用于幂等性处理，防止同一消息被重复处理
    ///   - 对应支付宝回调参数中的 `msg_id` 字段
    ///
    /// * `biz_content` - 消息的业务数据
    ///   - 包含具体的业务字段，如订单号、交易金额、交易状态等
    ///   - 对应支付宝回调参数中的 `biz_content` 字段
    ///   - 需要手动反序列化为具体的数据结构后再使用
    ///
    fn on_message<'a>(
        &'a self,
        msg_api: &str,
        msg_id: &str,
        biz_content: &str,
    ) -> BoxFuture<'a, ()>;
}
