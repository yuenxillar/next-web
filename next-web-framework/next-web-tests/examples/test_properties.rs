use std::collections::HashMap;

use axum::response::IntoResponse;
use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{
    application::Application, middleware::find_singleton::FindSingleton, Properties, Singleton,
};

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/properties", axum::routing::get(req_properties))
            .route("/serverPort", axum::routing::get(req_server_port))
            .route("/redisProperties", axum::routing::get(req_redis_properties))
            .route("/redisDynamicProperties", axum::routing::get(req_redis_dynamic_properties))
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
    format!("Server port: {:?}", properties.one_value::<u32>("next.server.port").unwrap())
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
#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.data.redis")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TestRedisProperties {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub password: Option<String>,
    pub database: Option<u8>,
}

#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.data.redis.dynamic", dynamic)]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TestDynamicRedisProperties {
    /// This is necessary, try not to change it as much as possible
    /// 这是必要的，尽量不要改变它, 后面的字段可以自定义
    pub base: HashMap<String, TestRedisProperties>,
   
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
