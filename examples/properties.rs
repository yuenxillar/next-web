use next_web::{
    Application, NextWebApplication,
    extract::find_singleton::FindSingleton,
    macros::{autoconfigure::configuration_properties, bind::get_mapping},
};

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[get_mapping(path = "/properties")]
async fn req_properties(
    FindSingleton(server): FindSingleton<TestServerProperties>,
    FindSingleton(redis): FindSingleton<TestRedisProperties>,
) -> impl IntoResponse {
    format!("server: {:?}\nredis: {:?}", server, redis)
}

#[get_mapping(path = "/serverPort")]
async fn req_server_port(
    FindSingleton(properties): FindSingleton<TestServerProperties>,
) -> impl IntoResponse {
    format!("Server port: {:?}", properties.port)
}

#[get_mapping(path = "/redisProperties")]
async fn req_redis_properties(
    FindSingleton(properties): FindSingleton<TestRedisProperties>,
) -> impl IntoResponse {
    format!("{:?}", properties)
}

#[get_mapping(path = "/redisDynamicProperties")]
async fn req_redis_dynamic_properties(
    FindSingleton(properties): FindSingleton<TestDynamicRedisProperties>,
) -> impl IntoResponse {
    format!("{:?}", properties)
}

#[configuration_properties(prefix = "next.server")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TestServerProperties {
    pub port: Option<u16>,
    pub address: Option<String>,
}

#[configuration_properties(prefix = "next.data.redis")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TestRedisProperties {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub password: Option<String>,
    pub database: Option<u8>,
}

#[configuration_properties(prefix = "next.data.redis.dynamic", dynamic)]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TestDynamicRedisProperties {}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
