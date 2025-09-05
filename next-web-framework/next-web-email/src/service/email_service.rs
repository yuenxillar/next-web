use std::{borrow::Cow, error::Error, path::Path};

use lettre::{
    message::{
        header::{self, ContentType},
        MessageBuilder, MultiPart, SinglePart,
    },
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use next_web_core::traits::{service::Service, singleton::Singleton};

use crate::{core::email_content::EmailContent, properties::email_properties::EmailProperties};

/// 邮件 Service
#[derive(Clone)]
pub struct EmailService {
    properties: EmailProperties,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}


impl Singleton  for EmailService {}
impl Service    for EmailService {}

impl EmailService {

    /// 创建新的邮件服务实例
    pub fn new(properties: EmailProperties) -> Result<Self, Box<dyn Error>> {
        let var = properties.clone();

        let mut transport = if properties.tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&var.host.unwrap_or_default())?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&var.host.unwrap_or_default())
        };

        transport = transport
            .port(var.port.unwrap_or_default())
            .credentials(Credentials::new(
                var.username.unwrap_or_default(),
                var.password.unwrap_or_default(),
            ));

        Ok(Self {
            properties,
            transport: transport.build(),
        })
    }

    /// 发送邮件
    pub async fn send(&self, content: EmailContent) -> Result<(), Box<dyn Error>> {
        // 构建发件人
        let from = if let Some(name) = &self.properties.from_name {
            format!("{} <{}>", name, self.properties.from.clone().unwrap())
        } else {
            self.properties.from.clone().unwrap()
        };

        // 开始构建邮件
        let mut builder = MessageBuilder::new()
            .from(from.parse()?)
            .subject(content.subject);

        // 添加收件人
        for to in content.to {
            builder = builder.to(to.parse()?);
        }

        // 添加抄送
        for cc in content.cc {
            builder = builder.cc(cc.parse()?);
        }

        // 添加密送
        for bcc in content.bcc {
            builder = builder.bcc(bcc.parse()?);
        }

        // 设置内容类型
        let content_type = if content.is_html {
            ContentType::TEXT_HTML
        } else {
            ContentType::TEXT_PLAIN
        };

        // 添加正文
        builder = builder.header(content_type);

        // 添加附件
        let mut multipart = MultiPart::mixed().build();
        for attachment in content.attachments {
            let path = Path::new(attachment.as_ref());
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("Invalid attachment filename")?;

            let data = std::fs::read(path)?;
            let part = SinglePart::builder()
                .header(header::ContentDisposition::attachment(filename))
                .body(data);
            multipart = multipart.singlepart(part);
        }

        // 设置正文内容
        let email = builder.multipart(multipart)?;

        // 发送邮件
        self.transport.send(email).await?;

        Ok(())
    }

    /// 发送简单文本邮件
    pub async fn send_simple<E: Into<Cow<'static, str>>>(
        &self,
        to: E,
        subject: E,
        content: E,
    ) -> Result<(), Box<dyn Error>> {
        let email_content = EmailContent {
            to: vec![to.into()],
            cc: vec![],
            bcc: vec![],
            subject: subject.into(),
            data: content.into(),
            is_html: false,
            attachments: vec![],
        };

        self.send(email_content).await
    }

    /// 发送 HTML 邮件
    pub async fn send_html<E: Into<Cow<'static, str>>>(
        &self,
        to: E,
        subject: E,
        content: E,
    ) -> Result<(), Box<dyn Error>> {
        let email_content = EmailContent {
            to: vec![to.into()],
            cc: vec![],
            bcc: vec![],
            subject: subject.into(),
            data: content.into(),
            is_html: true,
            attachments: vec![],
        };

        self.send(email_content).await
    }
}
