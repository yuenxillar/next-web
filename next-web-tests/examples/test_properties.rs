use std::collections::HashMap;

use axum::response::IntoResponse;
use next_web::{
    application::Application,
    extract::find_singleton::FindSingleton,
    macros::{autoconfigure::properties, bind::singleton},
};
use next_web_core::{ApplicationContext, async_trait, context::properties::ApplicationProperties};

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    /// initialize the middleware.
    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) {
    }

    // get the application router. (open api  and private api)
    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/properties", axum::routing::get(req_properties))
            .route("/serverPort", axum::routing::get(req_server_port))
            .route("/redisProperties", axum::routing::get(req_redis_properties))
            .route(
                "/redisDynamicProperties",
                axum::routing::get(req_redis_dynamic_properties),
            )
    }
}

async fn req_properties(
    FindSingleton(properties): FindSingleton<ApplicationProperties>,
) -> impl IntoResponse {
    format!("properties: {:?}", properties)
}

async fn req_server_port(
    FindSingleton(properties): FindSingleton<ApplicationProperties>,
) -> impl IntoResponse {
    format!(
        "Server port: {:?}",
        properties.get_value::<u32>("next.server.port").unwrap()
    )
}

async fn req_redis_properties(
    FindSingleton(properties): FindSingleton<TestRedisProperties>,
) -> impl IntoResponse {
    format!("{:?}", properties)
}

async fn req_redis_dynamic_properties(
    FindSingleton(properties): FindSingleton<TestDynamicRedisProperties>,
) -> impl IntoResponse {
    format!("{:?}", properties)
}

// 示例 用于获取配置文件的参数值
// Example, used to obtain parameter values for configuration files
#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.data.redis")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TestRedisProperties {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub password: Option<String>,
    pub database: Option<u8>,
}

#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.data.redis.dynamic", dynamic)]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TestDynamicRedisProperties {
    /// This is necessary, try not to change it as much as possible
    /// 这是必要的，尽量不要改变它, 后面的字段可以自定义
    pub dynamic: HashMap<String, TestRedisProperties>,
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
