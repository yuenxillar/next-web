use std::any::{Any, TypeId};

use next_web_core::traits::event::application_event_publisher::ApplicationEventPublisher;
use next_web_core::{
    async_trait,
    context::properties::ApplicationProperties,
    traits::event::{
        application_event::ApplicationEvent, application_listener::ApplicationListener,
    },
    ApplicationContext,
};

use next_web_dev::{
    application::Application,
    event::default_application_event_publisher::DefaultApplicationEventPublisher,
    util::local_date_time::LocalDateTime, Singleton,
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
            .get_single_with_name::<DefaultApplicationEventPublisher>("defaultApplicationEventPublisher")
            .to_owned();
        tokio::spawn(async move {
            loop {
                let event = TestEvent(LocalDateTime::timestamp());
                publisher.publish_event("", event).ok();

                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
            }
        });
        axum::Router::new().route("/", axum::routing::get(|| async move { "Hello, world!" }
    ))
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
    fn id(&self) -> &'static str {
        ""
    }

    fn event_id(&self) -> TypeId {
        TypeId::of::<TestEvent>()
    }

    async fn on_application_event(&mut self, event: &Box<dyn ApplicationEvent>) {
        let any: &dyn Any = event.as_ref();
        let e = any.downcast_ref::<TestEvent>().unwrap();
        println!("Time tick: {}", e.0)
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
