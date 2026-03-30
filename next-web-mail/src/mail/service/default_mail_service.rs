use std::{collections::HashMap, ops::Deref};

use lettre::{AsyncSmtpTransport, Tokio1Executor, transport::smtp::authentication::Credentials};
use next_web_core::{
    async_trait, error::BoxError, impl_service,
    mime_type::configurable_mime_file_type_map::ConfigurableMimeFileTypeMap,
};

use crate::mail::{
    autoconfigure::mail_properties::MailProperties, default_mail_message::DefaultMailMessage,
    mail_error::MailError, mail_sender::MailSender, mail_service::MailService,
    mime_message::MimeMessage,
};

/// DefaultMailService
#[derive(Clone)]
pub struct DefaultMailService {
    /// Mail Properties
    mail_properties: MailProperties,

    /// Session properties.
    properties: Option<HashMap<String, String>>,

    /// SMTP Transport
    smtp_transport: AsyncSmtpTransport<Tokio1Executor>,

    /// MimeFileTypeMap
    default_file_type_map: ConfigurableMimeFileTypeMap,
}

impl DefaultMailService {
    /// 创建新的邮件服务实例
    pub fn new(mut mail_properties: MailProperties) -> Result<Self, BoxError> {
        let MailProperties {
            host,
            port,
            username,
            password,
            protocol,
            default_encoding,
            properties,
            ssl,
        } = &mail_properties;
        let transport = if ssl.enabled() {
            AsyncSmtpTransport::<Tokio1Executor>::relay(host.as_deref().unwrap_or_default())?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(
                host.as_deref().unwrap_or_default(),
            )
        };

        let smtp_transport = transport
            .port(*port)
            .credentials(Credentials::new(
                username.as_ref().map(Clone::clone).unwrap_or_default(),
                password.as_ref().map(Clone::clone).unwrap_or_default(),
            ))
            .build();

        let mut default_file_type_map = ConfigurableMimeFileTypeMap::default();
        default_file_type_map.load_file("", None)?;

        let properties = mail_properties.properties.take();
        Ok(Self {
            mail_properties,
            properties,
            smtp_transport,
            default_file_type_map,
        })
    }

    /// 发送邮件
    async fn do_send<T>(&self, messages: T) -> Result<(), MailError>
    where
        T: IntoIterator<Item = DefaultMailMessage>,
    {
        messages.into_iter().map(|msg| {});

        // // 构建发件人
        // let from = if let Some(name) = &self.properties.from_name {
        //     format!("{} <{}>", name, self.properties.from.clone().unwrap())
        // } else {
        //     self.properties.from.clone().unwrap()
        // };

        // // 开始构建邮件
        // let mut builder = MessageBuilder::new()
        //     .from(from.parse()?)
        //     .subject(content.subject);

        // // 添加收件人
        // for to in content.to {
        //     builder = builder.to(to.parse()?);
        // }

        // // 添加抄送
        // for cc in content.cc {
        //     builder = builder.cc(cc.parse()?);
        // }

        // // 添加密送
        // for bcc in content.bcc {
        //     builder = builder.bcc(bcc.parse()?);
        // }

        // // 设置内容类型
        // let content_type = if content.is_html {
        //     ContentType::TEXT_HTML
        // } else {
        //     ContentType::TEXT_PLAIN
        // };

        // // 添加正文
        // builder = builder.header(content_type);

        // // 添加附件
        // let mut multipart = MultiPart::mixed().build();
        // for attachment in content.attachments {
        //     let path = Path::new(attachment.as_ref());
        //     let filename = path
        //         .file_name()
        //         .and_then(|name| name.to_str())
        //         .ok_or("Invalid attachment filename")?;

        //     let data = std::fs::read(path)?;
        //     let part = SinglePart::builder()
        //         .header(header::ContentDisposition::attachment(filename))
        //         .body(data);
        //     multipart = multipart.singlepart(part);
        // }

        // // 设置正文内容
        // let email = builder.multipart(multipart)?;

        // 发送邮件

        Ok(())
    }

    // /// 发送简单文本邮件
    // pub async fn send_simple<E: Into<Cow<'static, str>>>(
    //     &self,
    //     to: E,
    //     subject: E,
    //     content: E,
    // ) -> Result<(), Box<dyn Error>> {
    //     let email_content = EmailContent {
    //         to: vec![to.into()],
    //         cc: vec![],
    //         bcc: vec![],
    //         subject: subject.into(),
    //         data: content.into(),
    //         is_html: false,
    //         attachments: vec![],
    //     };

    //     self.send(email_content).await
    // }

    // /// 发送 HTML 邮件
    // pub async fn send_html<E: Into<Cow<'static, str>>>(
    //     &self,
    //     to: E,
    //     subject: E,
    //     content: E,
    // ) -> Result<(), Box<dyn Error>> {
    //     let email_content = EmailContent {
    //         to: vec![to.into()],
    //         cc: vec![],
    //         bcc: vec![],
    //         subject: subject.into(),
    //         data: content.into(),
    //         is_html: true,
    //         attachments: vec![],
    //     };

    //     self.send(email_content).await
    // }
}

#[async_trait]
impl MailService for DefaultMailService {
    fn create_mime_message(&self) -> MimeMessage {
        todo!()
    }

    async fn send_mime_message(&self, mime_message: MimeMessage) -> Result<(), MailError> {
        todo!()
    }

    async fn send_mime_messages(&self, mime_messages: Vec<MimeMessage>) -> Result<(), MailError> {
        todo!()
    }
}

#[async_trait]
impl MailSender for DefaultMailService {
    async fn send(&self, message: DefaultMailMessage) -> Result<(), MailError> {
        Ok(())
    }

    async fn send_batch(&self, messages: Vec<DefaultMailMessage>) -> Result<(), MailError> {
        Ok(())
    }
}

impl Deref for DefaultMailService {
    type Target = AsyncSmtpTransport<Tokio1Executor>;

    fn deref(&self) -> &Self::Target {
        &self.smtp_transport
    }
}

impl_service!(DefaultMailService);
