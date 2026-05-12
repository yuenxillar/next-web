# 支付宝支付集成库

> 支付宝支付客户端实现，提供完整的支付接口和回调处理。

## 📦 功能说明

本库封装了支付宝支付的常用场景，为商户应用提供开箱即用的支付能力：

| 功能 | 说明 |
|:---|:---|
| **扫码支付** | 商户生成二维码，用户扫码后完成支付 |
| **订单查询** | 根据商户订单号查询支付状态 |
| **网站支付** | 跳转支付宝页面完成支付（PC/移动端 H5） |
| **异步通知** | 接收支付宝服务器的支付结果回调 |

## 技术栈

- **HTTP Client**: `reqwest`
- **日志**: `tracing`
- **异步运行时**: `tokio`

## 使用前提

1. 申请支付宝开放平台应用，获取 `app_id`、商户私钥等凭证
2. 准备好支付宝证书文件（根证书、应用公钥证书、支付宝公钥证书）
3. 配置异步通知 URL（需公网可访问）和同步跳转 URL

```rust

use std::{collections::BTreeMap, error::Error};

use axum::{
    Form,
    extract::{Path, Query},
    response::Html,
};
use futures::FutureExt;
use next_web::{
    application::Application,
    extract::{find_singleton::FindSingleton, typed_header::TypedHeader},
    macros::bind::{get_mapping, post_mapping},
    rand::{self, Rng},
};
use next_web_core::{
    ApplicationContext, async_trait, context::properties::ApplicationProperties, headers::Host,
};
use next_web_pay_alipay::{
    config::AlipayConfig,
    payment::{
        common::{
            AlipayTradePay,
            model::{
                AlipayTradePayNotify, AlipayTradePayRequest, AlipayTradePrecreateRequest,
                AlipayTradeQueryRequest,
            },
        },
        page::{TradePagePay, model::AlipayTradePagePayRequest},
    },
    service::AliPayService,
};

#[derive(Default, Clone)]
pub struct TestApplication;

const MY_GATEWAY_URL: &str = "https://xxxxx";

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    #[allow(unused_variables)]
    async fn init_middleware(
        &self,
        ctx: &mut ApplicationContext,
        properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn on_ready(&self, ctx: &mut ApplicationContext) -> Result<(), Box<dyn Error>> {

        // 这里是使用的沙箱环境
        let mut config = AlipayConfig::default()
            .with_gateway_url("https://openapi-sandbox.dl.alipaydev.com/gateway.do")
            .with_notify_url(format!("{MY_GATEWAY_URL}/pay/notify/alipay"));

        macro_rules! auto_set {
            ($field:ident, path: $path:expr) => {
                if let Some(home) = std::env::home_dir() {
                    let full_path = home.join($path);
                    if let Some(path_str) = full_path.to_str() {
                        config.$field(path_str.to_string());
                    } else {
                        config.$field($path.to_string());
                    }
                } else {
                    config.$field($path.to_string());
                }
            };
        }

        auto_set!(set_alipay_root_cert_path, path: "alipay/cert/alipayRootCert.crt");
        auto_set!(set_merchant_cert_path, path: "alipay/cert/appPublicCert.crt");
        auto_set!(set_alipay_cert_path, path: "alipay/cert/alipayPublicCert.crt");

        let alipay_service = AliPayService::new(config);

        ctx.insert_singleton_with_default_name(alipay_service);
        Ok(())
    }
}

#[post_mapping(path = "/trade/pay/{auth_code}")]
async fn trade_pay(
    FindSingleton(alipay_service): FindSingleton<AliPayService>,
    Path(auth_code): Path<String>,
) -> String {
    let req = AlipayTradePayRequest::new(generate_order_no(), "0.05", "Iphone 18 512GB", auth_code);

    let resp = match alipay_service.pay(req).await {
        Ok(resp) => resp,
        Err(err) => return format!("Pay failed: {}", err.to_string()).into_response(),
    };
    println!("{:?}", resp);

    "Ok".to_string()
}

#[post_mapping(path = "/trade/precreate")]
async fn precreate(FindSingleton(alipay_service): FindSingleton<AliPayService>) -> String {
    let req =
        AlipayTradePrecreateRequest::new(generate_order_no(), "0.05", "Iphone 18 512GB", None);

    let resp = match alipay_service.precreate(req).await {
        Ok(resp) => resp,
        Err(err) => return format!("Precreate failed: {}", err.to_string()).into_response(),
    };

    if resp.is_success() {
        resp.data().qr_code.unwrap_or_default()
    } else {
        "Unknown".to_string()
    }
}

#[get_mapping(path = "/trade/query/{out_trade_no}")]
async fn trade_query(
    FindSingleton(alipay_service): FindSingleton<AliPayService>,
    Path(out_trade_no): Path<String>,
) -> String {
    let req = AlipayTradeQueryRequest::with_out_trade_no(out_trade_no);
    alipay_service
        .query(req)
        .map(|s| s.ok().map(|e| format!("{:?}", e)))
        .await
        .unwrap_or("Unknown".into())
}

#[post_mapping(path = "/trade/page_pay")]
async fn page_pay(FindSingleton(alipay_service): FindSingleton<AliPayService>) -> String {
    let req = AlipayTradePagePayRequest::new(
        generate_order_no(),
        "0.05",
        "Iphone 18 512GB",
        format!("{MY_GATEWAY_URL}/trade/page_pay/return"),
    );

    let resp = match alipay_service.page_pay(req).await {
        Ok(resp) => resp,
        Err(err) => return format!("Page Pay failed: {}", err.to_string()).into_response(),
    };

    Html(resp)
}

#[get_mapping(path = "/trade/page_pay/return")]
async fn alipay_return(Query(params): Query<BTreeMap<String, String>>) -> String {
    println!("{:?}", params);

    "success".to_string()
}

#[post_mapping(path = "/pay/notify/alipay")]
async fn alipay_notify(
    FindSingleton(alipay_service): FindSingleton<AliPayService>,
    TypedHeader(host): TypedHeader<Host>,
    // 建议先用有序的 map 存储参数
    Form(trade_pay_notify): Form<BTreeMap<String, String>>,
) -> &'static str {
    // 1. 商家需要验证该通知数据中的 out_trade_no 是否为商家系统中创建的订单号。
    // 2. 判断 total_amount 是否确实为该订单的实际金额（即商家订单创建时的金额）。
    // 3. 校验通知中的 seller_id（或者 seller_email ) 是否为 out_trade_no 这笔单据的对应的操作方（有的时候，一个商家可能有多个seller_id/seller_email）。
    // 4. 验证 app_id 是否为该商家本身。
    // 5. 上述 1、2、3、4 有任何一个验证不通过，则表明本次通知是异常通知，务必忽略。
    // 在上述验证通过后商家必须根据支付宝不同类型的业务通知，正确的进行不同的业务处理，并且过滤重复的通知结果数据。在支付宝的业务通知中，只有交易通知状态为 TRADE_SUCCESS 或 TRADE_FINISHED 时，支付宝才会认定为买家付款成功。

    // 可以转换为 AlipayTradePayNotify
    println!(
        "trade_pay_notify: {:?}",
        serde_json::to_string(&trade_pay_notify)
            .map(|json_str| serde_json::from_str::<AlipayTradePayNotify>(json_str.as_str()).ok())
            .ok()
            .unwrap_or_default()
    );

    // 1. 判断来源是否为阿里云官方
    if host.hostname() == "xxxx" {
        return "fail".into_response();
    }

    trade_pay_notify
        .get("sign")
        .map(|sign| alipay_service.verify_sign(&trade_pay_notify, sign.as_str()))
        .unwrap_or_default()
        .then(|| "success")
        .unwrap_or("fail")
}

fn generate_order_no() -> String {
    let mut rng = rand::thread_rng();
    let random_part: u64 = rng.gen_range(10000000000000..99999999999999);
    format!("{}{}", "20150320", random_part)
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

```