# Next Web Dev

Dev - make everything simpler

Current:  
[Axum](https://crates.io/crates/axum)   as a web server   
[Rudi](https://crates.io/crates/rudi)   as a dependency injection  

And many other excellent crates

```rust

use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{atomic::AtomicU32, Arc},
};

use axum::{extract::ConnectInfo, response::IntoResponse, routing::get};
use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{
    application::Application, middleware::find_singleton::FindSingleton, Singleton,
};
use next_web_macros::Find;
use tokio::sync::Mutex;
use tracing::info;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    async fn before_start(&mut self, ctx: &mut ApplicationContext) {
        ctx.insert_singleton_with_name(Arc::new(AtomicU32::new(0)), "requestCount");
        ctx.insert_singleton_with_name(Arc::new(Mutex::new(HashSet::<SocketAddr>::new())),"requestIps");

        // 在这里将实例插入至 ctx 之中
        // Insert the instance into the ctx here
        ctx.insert_singleton_with_name(ApplicationStore::default(), "applicationStoreTwo");
    }

    async fn application_router(&mut self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/hello", get(req_hello))
            .route("/record", get(req_record))
            .route("/recordTwo", get(req_record_two))
    }
}

async fn req_hello() -> impl IntoResponse {
    " Hello Axum! \n Hello Next Web!"
}

async fn req_record(
    // 这里将根据泛型的具体类型进行查找 [ApplicationStore -> applicationStore]
    // Here, we will search based on the specific types of generics [ApplicationStore -> applicationStore]
    FindSingleton(store): FindSingleton<ApplicationStore>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    store.add(addr).await;
    "Ok"
}

#[Find]
async fn req_record_two(
    // 这里使用变量名搜索单例 [application_store_two -> applicationStoreTwo]
    // Search for singleton using variable names  [application_store_two -> applicationStoreTwo]
    FindSingleton(application_store_two): FindSingleton<ApplicationStore>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    application_store_two.add(addr).await;
    "Ok"
}

#[Singleton(name = "applicationStore")]
#[derive(Clone)]
pub struct ApplicationStore {
    pub request_count: Arc<AtomicU32>,
    pub request_ips: Arc<Mutex<HashSet<SocketAddr>>>,
}

impl ApplicationStore {
    async fn add(&self, addr: SocketAddr) {
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        match self.request_ips.lock().await.insert(addr) {
            true => info!("Store add new ip: {}", addr.to_string()),
            false => info!("Ip already exists in store: {}", addr.to_string()),
        }

        info!(
            "Current request count: {}",
            self.request_count
                .load(std::sync::atomic::Ordering::Relaxed)
        )
    }
}

impl Default for ApplicationStore {
    fn default() -> Self {
        Self {
            request_count: Arc::new(AtomicU32::new(u32::MAX / 2)),
            request_ips: Default::default(),
        }
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

```