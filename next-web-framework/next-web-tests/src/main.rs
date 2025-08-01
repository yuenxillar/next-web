#![allow(missing_docs)]

use axum::Router;
use next_web_core::async_trait;
use next_web_core::utils::any_matcher::RouteMatcher;
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};

use next_web_dev::application::Application;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, _ctx: &mut ApplicationContext) -> axum::Router {
        // _ctx.contains_single_with_name
        Router::new()
    }
}

#[tokio::main]
async fn main() {
    // TestApplication::run().await;
    let mut route = RouteMatcher::new();
    
    route.insert("/test", 0).ok();
    route.insert("/test/*.js", 1).ok();
    route.insert("/test/**", 2).ok();

     route.insert("/789", 0).ok();
    route.insert("/789/*.css", 1).ok();
    route.insert("/789/**", 2).ok();

     route.insert("/test/698", 0).ok();
    route.insert("/test/6856/*.js", 1).ok();
    route.insert("/test/3333", 2).ok();

    println!("route: {}", route.at("/test/test.js").unwrap());
    // println!("route: {}", route.at("/test").unwrap());
    // println!("route: {}", route.at("/test/test666").unwrap());
}
