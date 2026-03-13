use next_web::traits::event::application_event_publisher::ApplicationEventPublisher;
use next_web::util::local_date_time::LocalDateTime;
use next_web::{
    event::default_application_event_publisher::DefaultApplicationEventPublisher,
    macros::event::event_listener,
};
use next_web_core::{
    async_trait,
    context::properties::ApplicationProperties,
    traits::event::{
        application_event::ApplicationEvent, application_listener::ApplicationListener,
    },
    ApplicationContext,
};

use next_web::{application::Application, macros::bind::singleton};

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
        let event_publisher = ctx
            .get_single_with_default_name::<DefaultApplicationEventPublisher>()
            .unwrap()
            .to_owned();

        tokio::spawn(async move {
            loop {
                event_publisher
                    .publish_event(TestEvent(LocalDateTime::timestamp()))
                    .await
                    .unwrap();

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }
}

#[singleton]
#[derive(Clone)]
pub struct TestListener;

#[singleton]
#[derive(Clone)]
pub struct TestEvent(i64);
impl ApplicationEvent for TestEvent {}

#[async_trait]
#[event_listener(id = "testListener")]
impl ApplicationListener<TestEvent> for TestListener {
    async fn on_application_event(&self, event: &TestEvent) {
        println!("Time tick: {}", event.0)
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
