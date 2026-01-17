use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{atomic::AtomicU32, Arc},
};

use next_web::{
    any_mapping, application::Application, extract::find_singleton::FindSingleton, get_mapping,
    post_mapping, request_mapping, util::local_date_time::LocalDateTime, Singleton,
};
use next_web::{
    async_trait, context::properties::ApplicationProperties, idempotency, ApplicationContext,
};
use next_web::{extract::ConnectInfo, traits::store::idempotency_store::IdempotencyStore};
use next_web::{
    response::{Html, IntoResponse},
    store::memory_idempotency_store::MemoryIdempotencyStore,
};
use tokio::sync::Mutex;
use tracing::info;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) {
    }

    async fn on_ready(&self, ctx: &mut ApplicationContext) {
        ctx.insert_singleton_with_name(Arc::new(AtomicU32::new(0)), "requestCount");

        #[rustfmt::skip]
        ctx.insert_singleton_with_name(Arc::new(Mutex::new(HashSet::<SocketAddr>::new())),"requestIps");
        ctx.insert_singleton_with_name(ApplicationStore::default(), "applicationStoreTwo");

        ctx.insert_singleton_with_name(
            Arc::new(MemoryIdempotencyStore::new()) as Arc<dyn IdempotencyStore<Value = ()>>,
            "memoryIdempotencyStore",
        );
    }
}

#[request_mapping(method = "GET", path = "/timestamp")]
pub async fn req_timestamp() -> impl IntoResponse {
    LocalDateTime::now()
}

#[get_mapping(path = "/hello")]
pub async fn req_hello() -> String {
    " Hello Axum! \n Hello Next Web!".to_string()
}

#[post_mapping(path = "/record")]
pub async fn req_record(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    FindSingleton(store): FindSingleton<ApplicationStore>,
) -> impl IntoResponse {
    store.add(addr).await;
    "Ok"
}

#[any_mapping(
    path = "/recordTwo",
    headers  = ["ContentType", "Authorization"],
    consume = "application/json",
    produce = "application/json"
)]
pub async fn req_record_two(
    // Search for singleton using variable names
    #[find] FindSingleton(application_store_two): FindSingleton<ApplicationStore>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<&'static str, ()> {
    application_store_two.add(addr).await;

    return Ok("{\"message\": \"Ok\"}");
}

#[allow(unused)]
struct TestUserRoutes;

#[request_mapping(path = "/user")]
impl TestUserRoutes {
    // Request -> /user/login
    #[get_mapping(path = "/login")]
    async fn req_login() -> impl IntoResponse {
        Html("<h1>Login Page</h1>")
    }

    // Request -> /user/logout
    #[idempotency(
        name = "memoryIdempotencyStore",
        key = "Idempotency-Key",
        cache_key_prefix = "test_key",
        ttl = 10
    )]
    #[request_mapping(method = "POST", path = "/logout")]
    async fn req_logout() -> impl IntoResponse {
        Html("<h1>Logout Page</h1>")
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

#[cfg(test)]
mod tests {
    use next_web::util::aes::local_file_encrypt;

    #[test]
    fn test_local_file_encrypt() {
        match local_file_encrypt("2025/10/16/test") {
            Ok(_) => {}
            Err(error) => println!("Error encrypting file, case: {error}"),
        };
    }
}
