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

    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

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
pub struct TestDynamicRedisProperties {}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
