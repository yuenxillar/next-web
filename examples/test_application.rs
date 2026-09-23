use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

use next_web::{
    Application, NextWebApplication,
    extract::find_singleton::FindSingleton,
    macros::bind::{get_mapping, singleton},
};

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[get_mapping(path = "/hello")]
async fn hello() -> &'static str {
    "Hello, World!"
}

#[get_mapping(path = "/change")]
async fn change(FindSingleton(app_store): FindSingleton<AppStore>) -> &'static str {
    app_store.len.fetch_add(1, Ordering::Relaxed);
    app_store.active.fetch_not(Ordering::Relaxed);

    "Ok"
}

#[derive(Clone, Default)]
#[singleton(default)]
struct AppStore {
    pub len: Arc<AtomicU32>,
    pub active: Arc<AtomicBool>,
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
