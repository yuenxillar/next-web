use std::{collections::BTreeMap, error::Error};

use axum::{
    Form,
    extract::{Path, Query},
    response::Html,
};
use futures::FutureExt;
use next_web::{
    application::Application,
    extract::find_singleton::FindSingleton,
    macros::bind::{get_mapping, post_mapping},
    rand::{self, Rng},
};
use next_web_core::{ApplicationContext, async_trait, context::properties::ApplicationProperties};
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

const MY_GATEWAY_URL: &str = "https://rom-brochure-seo-controlled.trycloudflare.com";

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
        let mut config = AlipayConfig::default()
            .with_gateway_url("https://openapi-sandbox.dl.alipaydev.com/gateway.do")
            .with_notify_url(format!("{MY_GATEWAY_URL}/pay/notify/alipay"));
        config.set_alipay_root_cert_path(
            std::env::home_dir()
                .map(|s| {
                    s.join("alipay/cert/alipayRootCert.crt")
                        .to_str()
                        .map(ToString::to_string)
                })
                .unwrap()
                .unwrap_or("/alipay/cert/alipayRootCert.crt".into()),
        );
        config.set_merchant_cert_path(
            std::env::home_dir()
                .map(|s| {
                    s.join("alipay/cert/appPublicCert.crt")
                        .to_str()
                        .map(ToString::to_string)
                })
                .unwrap()
                .unwrap_or("/alipay/cert/appPublicCert.crt".into()),
        );
        config.set_alipay_cert_path(
            std::env::home_dir()
                .map(|s| {
                    s.join("alipay/cert/alipayPublicCert.crt")
                        .to_str()
                        .map(ToString::to_string)
                })
                .unwrap()
                .unwrap_or("/alipay/cert/alipayPublicCert.crt".into()),
        );

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
    // 建议先用有序的 map 存储参数
    Form(trade_pay_notify): Form<BTreeMap<String, String>>,
) -> String {
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

    trade_pay_notify
        .get("sign")
        .map(|sign| alipay_service.verify_sign(&trade_pay_notify, sign.as_str()))
        .unwrap_or_default()
        .then(|| String::from("success"))
        .unwrap_or(String::from("fail"))
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
