use std::any::{Any, TypeId};

use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{
    application::Application,
    event::{
        application_event::ApplicationEvent,
        application_event_publisher::ApplicationEventPublisher,
        application_listener::ApplicationListener,
        default_application_event_publisher::DefaultApplicationEventPublisher,
    },
    util::local_date_time::LocalDateTime,
    Singleton,
};

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> axum::Router {
        let publisher = ctx
            .get_single::<DefaultApplicationEventPublisher>()
            .to_owned();
        tokio::spawn(async move {
            loop {
                let event = TestEvent(LocalDateTime::timestamp());
                publisher.publish_event("", Box::new(event)).ok();

                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
            }
        });
        axum::Router::new()
    }
}

#[Singleton(binds=[Self::into_listener])]
#[derive(Clone)]
pub struct TestListener;

impl TestListener {
    fn into_listener(self) -> Box<dyn ApplicationListener> {
        Box::new(self)
    }
}

#[Singleton]
#[derive(Clone)]
pub struct TestEvent(i64);

impl ApplicationEvent for TestEvent {}

#[async_trait]
impl ApplicationListener for TestListener {
    fn event_id(&self) -> TypeId {
        TypeId::of::<TestEvent>()
    }

    async fn on_application_event(&mut self, event: &Box<dyn ApplicationEvent>) {
        let any: &dyn Any = event.as_ref();
        let e = any.downcast_ref::<TestEvent>().unwrap();
        println!("time tick: {}", e.0)
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
