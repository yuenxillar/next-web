use std::borrow::Cow;

/// 邮件内容
#[derive(Debug, Clone)]
pub struct EmailContent {
    /// 收件人
    pub to: Vec<Cow<'static, str>>,
    /// 抄送
    pub cc: Vec<Cow<'static, str>>,
    /// 密送
    pub bcc: Vec<Cow<'static, str>>,
    /// 主题
    pub subject: Cow<'static, str>,
    /// 内容
    pub data: Cow<'static, str>,
    /// 是否是 HTML 内容
    pub is_html: bool,
    /// 附件路径
    pub attachments: Vec<Cow<'static, str>>,
}