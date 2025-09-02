use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{atomic::AtomicU32, Arc},
};

use tracing::info;
use axum::{extract::ConnectInfo, response::IntoResponse, routing::get};
use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{
    application::Application, middleware::find_singleton::FindSingleton, Singleton,
};
use tokio::sync::Mutex;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    async fn before_start(&mut self, ctx: &mut ApplicationContext) {
        ctx.insert_singleton_with_name(Arc::new(AtomicU32::new(0)), "requestCount");
        ctx.insert_singleton_with_name(Arc::new(Mutex::new(HashSet::<SocketAddr>::new())), "requestIps");
    }

    async fn application_router(&mut self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/hello", get(req_hello))
            .route("/record", get(req_record))
    }
}

async fn req_hello() -> impl IntoResponse {
    " Hello Axum! \n Hello Next Web!"
}

async fn req_record(
    FindSingleton(store): FindSingleton<ApplicationStore>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>
) -> impl IntoResponse {
    store.add(addr).await;
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
        self.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        match self.request_ips.lock().await.insert(addr) {
            true => info!("Store add new ip: {}", addr.to_string()),
            false => info!("Ip already exists in store: {}", addr.to_string())
        }   

        info!("Current request count: {}", self.request_count.load(std::sync::atomic::Ordering::Relaxed))     
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
