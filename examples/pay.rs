use std::{collections::BTreeMap, error::Error};

use axum::{
    Form,
    extract::{Path, Query},
    response::Html,
};
use next_web::{
    Application, NextWebApplication,
    extract::{find_singleton::FindSingleton, typed_header::TypedHeader},
    macros::bind::{get_mapping, post_mapping, singleton},
    rand::{self, Rng},
};
use next_web_context::ApplicationContextExt;
use next_web_core::{
    ApplicationContext, Ordered, async_trait, headers::Host,
    traits::config::auto_configuration::AutoConfiguration,
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

#[derive(Default)]
pub struct TestApplication;

const MY_GATEWAY_URL: &str = "https://rom-brochure-seo-controlled.trycloudflare.com";

impl Application for TestApplication {}

/// Creates the Alipay service of the application.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct AlipayAutoConfiguration;

impl AlipayAutoConfiguration {
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

impl Ordered for AlipayAutoConfiguration {
    fn order(&self) -> i32 {
        0
    }
}

#[async_trait]
impl AutoConfiguration for AlipayAutoConfiguration {
    /// Registers the Alipay service of the application.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the service is registered in.
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
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

    match alipay_service.pay(req).await {
        Ok(resp) => {
            println!("{resp:?}");

            "Ok".to_string()
        }
        Err(error) => format!("Pay failed: {error}"),
    }
}

#[post_mapping(path = "/trade/precreate")]
async fn precreate(FindSingleton(alipay_service): FindSingleton<AliPayService>) -> String {
    let req =
        AlipayTradePrecreateRequest::new(generate_order_no(), "0.05", "Iphone 18 512GB", None);

    match alipay_service.precreate(req).await {
        Ok(resp) if resp.is_success() => resp.data().qr_code.unwrap_or_default(),
        Ok(_) => "Unknown".to_string(),
        Err(error) => format!("Precreate failed: {error}"),
    }
}

#[get_mapping(path = "/trade/query/{out_trade_no}")]
async fn trade_query(
    FindSingleton(alipay_service): FindSingleton<AliPayService>,
    Path(out_trade_no): Path<String>,
) -> String {
    let req = AlipayTradeQueryRequest::with_out_trade_no(out_trade_no);

    match alipay_service.query(req).await {
        Ok(response) if response.is_success() => format!("{:?}", response.data()),
        Ok(response) => response.to_error_string(),
        Err(error) => format!("Query failed: {error}"),
    }
}

#[post_mapping(path = "/trade/page_pay")]
async fn page_pay(FindSingleton(alipay_service): FindSingleton<AliPayService>) -> Response {
    let req = AlipayTradePagePayRequest::new(
        generate_order_no(),
        "0.05",
        "Iphone 18 512GB",
        format!("{MY_GATEWAY_URL}/trade/page_pay/return"),
    );

    match alipay_service.page_pay(req).await {
        Ok(resp) => Html(resp).into_response(),
        Err(error) => format!("Page Pay failed: {error}").into_response(),
    }
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
    let verified = host.hostname() != "xxxx"
        && trade_pay_notify
            .get("sign")
            .map(|sign| alipay_service.verify_sign(&trade_pay_notify, sign.as_str()))
            .unwrap_or_default();

    if verified { "success" } else { "fail" }
}

fn generate_order_no() -> String {
    let mut rng = rand::thread_rng();
    let random_part: u64 = rng.gen_range(10000000000000..99999999999999);
    format!("{}{}", "20150320", random_part)
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
