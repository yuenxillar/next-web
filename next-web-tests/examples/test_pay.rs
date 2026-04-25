use std::error::Error;

use axum::extract::Path;
use next_web::{
    application::Application,
    extract::find_singleton::FindSingleton,
    macros::bind::post_mapping,
    rand::{self, Rng},
    signal::APPLICATION_STARTED_SIGNAL,
};
use next_web_core::{ApplicationContext, async_trait, context::properties::ApplicationProperties};
use next_web_pay_alipay::{
    config::AlipayConfig,
    payment::{
        TradePay,
        trade::model::{TradePayRequest, TradePrecreateRequest},
    },
    service::AliPayService,
};

#[derive(Default, Clone)]
pub struct TestApplication;

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
        let config = AlipayConfig::default()
            .with_gateway_url("https://openapi-sandbox.dl.alipaydev.com/gateway.do");

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
    let req = TradePayRequest::new(generate_order_no(), "99.99", "Iphone 18 512GB", auth_code);

    let resp = match alipay_service.pay(req).await {
        Ok(resp) => resp,
        Err(err) => return format!("Pay failed: {}", err.to_string()).into_response(),
    };

    println!("{:?}", resp);

    "Ok".to_string()
}

#[post_mapping(path = "/trade/precreate")]
async fn precreate(FindSingleton(alipay_service): FindSingleton<AliPayService>) -> String {
    let req = TradePrecreateRequest::new(generate_order_no(), "99.99", "Iphone 18 512GB", None);

    let resp = match alipay_service.precreate(req).await {
        Ok(resp) => resp,
        Err(err) => return format!("Precreate failed: {}", err.to_string()).into_response(),
    };

    println!("{:?}", resp);

    "Ok".to_string()
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
