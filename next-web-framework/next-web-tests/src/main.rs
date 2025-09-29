use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{atomic::AtomicU32, Arc},
};

use next_web_dev::extract::ConnectInfo;
use next_web_dev::response::IntoResponse;
use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{
    application::Application, middleware::find_singleton::FindSingleton,
    util::local_date_time::LocalDateTime, AnyMapping, GetMapping, PostMapping, RequestMapping,
    Singleton,
};
use tokio::sync::Mutex;
use tracing::info;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    async fn init_middleware(&self, _properties: &ApplicationProperties) {}

    async fn before_start(&self, ctx: &mut ApplicationContext) {
        ctx.insert_singleton_with_name(Arc::new(AtomicU32::new(0)), "requestCount");

        #[rustfmt::skip]
        ctx.insert_singleton_with_name(Arc::new(Mutex::new(HashSet::<SocketAddr>::new())),"requestIps");
        ctx.insert_singleton_with_name(ApplicationStore::default(), "applicationStoreTwo");
    }
}

#[RequestMapping(method = "GET", path = "/timestamp")]
pub async fn req_timestamp() -> impl IntoResponse {
    LocalDateTime::now()
}

#[GetMapping(path = "/hello")]
pub async fn req_hello() -> impl IntoResponse {
    " Hello Axum! \n Hello Next Web!"
}

#[PostMapping(path = "/record")]
pub async fn req_record(
    FindSingleton(store): FindSingleton<ApplicationStore>,
    ConnectInfo(addr): axum::extract::ConnectInfo<SocketAddr> ,
) -> impl IntoResponse {
    store.add(addr).await;
    "Ok"
}

#[AnyMapping(
    path = "/recordTwo", 
    headers = ["Content-Type", "Authorization"],
    consumes = "application/json",
    produces = "application/json"
)]
pub async fn req_record_two(
    // Search for singleton using variable names
    #[find] FindSingleton(application_store_two): FindSingleton<ApplicationStore>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    application_store_two.add(addr).await;

    return "{\"message\": \"Ok\"}";
}

#[allow(unused)]
struct TestUserRoutes;

#[RequestMapping(path = "/user")]
impl TestUserRoutes {

    // Request -> /user/login
    #[GetMapping(path = "/login")]
    async fn req_login() -> impl IntoResponse {
        "Ok"
    }

    // Request -> /user/logout
    #[PostMapping(path = "/logout")]
    async fn req_logout() -> impl IntoResponse {
        "Ok"
    }
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
