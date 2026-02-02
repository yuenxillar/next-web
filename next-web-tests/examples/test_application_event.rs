use std::any::{Any, TypeId};
use std::sync::Arc;

use next_web::traits::event::application_event::EventId;
use next_web_core::traits::event::application_event_publisher::ApplicationEventPublisher;
use next_web_core::{
    async_trait,
    context::properties::ApplicationProperties,
    traits::event::{
        application_event::ApplicationEvent, application_listener::ApplicationListener,
    },
    ApplicationContext,
};

use next_web::{
    application::Application,
    event::default_application_event_publisher::DefaultApplicationEventPublisher,
    util::local_date_time::LocalDateTime, Singleton,
};

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    /// initialize the middleware.
    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) {
    }

    async fn on_ready(&self, ctx: &mut ApplicationContext) {
        let publisher = ctx
            .get_single_with_default_name::<DefaultApplicationEventPublisher>()
            .unwrap()
            .to_owned();
        tokio::spawn(async move {
            loop {
                let event = TestEvent(LocalDateTime::timestamp());
                publisher.publish_event(event).await.ok();

                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
            }
        });
    }
}

#[Singleton(binds=[Self::into_listener])]
#[derive(Clone)]
pub struct TestListener;

impl TestListener {
    fn into_listener(self) -> Arc<dyn ApplicationListener> {
        Arc::new(self)
    }
}

#[Singleton]
#[derive(Clone)]
pub struct TestEvent(i64);

impl ApplicationEvent for TestEvent {
    fn id(&self) -> String {
        String::default()
    }
}

#[async_trait]
impl ApplicationListener for TestListener {
    fn event_id(&self) -> EventId {
        (String::default(), TypeId::of::<TestEvent>())
    }

    async fn on_application_event(&self, event: &Box<dyn ApplicationEvent>) {
        let any: &dyn Any = event.as_ref();
        let e = any.downcast_ref::<TestEvent>().unwrap();
        println!("Time tick: {}", e.0)
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
