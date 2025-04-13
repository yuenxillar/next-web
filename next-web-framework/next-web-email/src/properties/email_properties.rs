use rudi::{Properties, Singleton};

#[Singleton(default, binds=[Self::into_properties])]
#[Properties( prefix = "next.email")]
#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct EmailProperties {
      /// SMTP 服务器地址
      pub host: Option<String>,
      /// SMTP 服务器端口
      pub port: Option<u16>,
      /// 用户名
      pub username: Option<String>,
      /// 密码
      pub password: Option<String>,
      /// 发件人邮箱
      pub from: Option<String>,
      /// 发件人名称
      pub from_name: Option<String>,
      /// 是否启用 TLS
      pub tls: bool,
}