
## 请检查是否存在以下环境变量

### 阿里云
- `ALIBABA_CLOUD_ACCESS_KEY_ID`
- `ALIBABA_CLOUD_ACCESS_KEY_SECRET`

### 腾讯云
- `TENCENTCLOUD_SECRET_ID`
- `TENCENTCLOUD_SECRET_KEY`


## 代码示例
```rust

use next_web_sms::aliyun::{respnose::sms_respnose::RespCode, service::aliyun_sms_service::AliyunCloudSmsService};
use next_web_sms::core::service::{sms_service::SmsService, template_service::TemplateService, sign_service::SignService};
use next_web_sms::aliyun::models::sms_template_respnose::create_respnose::CreateSmsTemplateRespnose;
use next_web_sms::aliyun::respnose::template_respnose::AliyunCloudTemplateResponse;
use next_web_sms::aliyun::respnose::sign_respnose::AliyunCloudSignResponse;
use next_web_sms::aliyun::models::sms_sign_respnose::create_respnose::CreateSmsSignRespnose;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // sms 短信
    // aliyun
    let aliyun_sms_service = AliyunCloudSmsService::new();
    let resp = aliyun_sms_service.send_sms("phone_number", "sign_name", "template_code", "template_param", None).await?;

    println!("send sms response: {:?}", resp);
    assert_eq!(resp.code, RespCode::Ok);

    // tencnet
    let tencnet_sms_service = TencentCloudSmsService::new();
     let resp1 = tencnet_sms_service.send_sms("phone_number", "sign_name", "template_code", "template_param", None).await?;
    println!("send sms response: {:?}", resp1);


    // template 模板
    let resp2: AliyunCloudTemplateResponse<CreateSmsTemplateRespnose> = aliyun_sms_service.create_template("template_name", "template_content", 0, None).await?;
    println!("create template response: {:?}", resp2);


    // sign 签名
    let resp3: AliyunCloudSignResponse<CreateSmsSignRespnose> = aliyun_sms_service.create_sign("sign_name", "sign_type", "sign_purpose", "qualification_id", "remark", None).await?;
    println!("create sign response: {:?}", resp3);

    Ok(())
}

```