use next_web::core::context::properties::ApplicationProperties;
use next_web::core::traits::event::application_event::EventAttributes;
use next_web::core::traits::event::application_listener::ApplicationListener;
use next_web::core::{ApplicationContext, async_trait};
use next_web::{application::Application, macros::bind::singleton};
use next_web::{
    event::default_application_event_publisher::DefaultApplicationEventPublisher,
    macros::event::event_listener,
};
use next_web_core::traits::event::application_event::ApplicationEvent;
use next_web_core::traits::event::application_event_publisher::ApplicationEventPublisher;

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
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn on_ready(
        &self,
        ctx: &mut ApplicationContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let event_publisher = ctx
            .get_single_with_default_name::<DefaultApplicationEventPublisher>()
            .unwrap()
            .to_owned();

        tokio::spawn(async move {
            loop {
                event_publisher
                    .publish_event(TestEvent {
                        attr: Default::default(),
                    })
                    .await
                    .unwrap();

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        Ok(())
    }
}

#[singleton]
#[derive(Clone)]
pub struct TestListener;

#[derive(Clone, Default)]
pub struct TestEvent {
    attr: EventAttributes,
}

// Now the timestamp will be automatically obtained
impl AsRef<EventAttributes> for TestEvent {
    fn as_ref(&self) -> &EventAttributes {
        &self.attr
    }
}

#[async_trait]
#[event_listener(id = "testListener")]
impl ApplicationListener<TestEvent> for TestListener {
    async fn on_application_event(&self, event: &TestEvent) {
        println!("Timestamp: {}", event.timestamp())
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
