
```rust

use next_web_sms::aliyun::{respnose::sms_respnose::RespCode, service::aliyun_sms_service::AliyunSmsService};
use next_web_sms::core::service::{sms_service::SmsService, template_service::TemplateService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // sms 短信
    // aliyun
    let aliyun_sms_service = AliyunSmsService::new();
    let resp = aliyun_sms_service.send_sms("phone_number", "sign_name", "template_code", "template_param", None).await?;

    println!("send sms response: {:?}", resp);
    assert_eq!(resp.code, RespCode::Ok);

    // tencnet
    let tencnet_sms_service = TencentCloudSmsService::new();
     let resp1 = tencnet_sms_service.send_sms("phone_number", "sign_name", "template_code", "template_param", None).await?;
    println!("send sms response: {:?}", resp1);


    // template 模板
    let resp2 = aliyun_sms_service.create_template("template_name", "template_content", 0, None).await?;
    println!("create template response: {:?}", resp2);
    Ok(())
}

```