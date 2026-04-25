use next_web::{
    application::Application,
    core::{
        ApplicationContext, async_trait,
        context::properties::ApplicationProperties,
        traits::{
            application::application_lifecycle::{ApplicationLifecycle, ShutdownContext},
            ordered::Ordered,
        },
    },
    macros::bind::singleton,
};
use tracing::info;

#[derive(Clone, Default)]
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
}

#[derive(Clone)]
#[allow(unused)]
#[singleton(binds = [Self::into_lifecycle])]
struct TestApplicationLifecycle;

impl TestApplicationLifecycle {
    pub fn into_lifecycle(self: Self) -> Box<dyn ApplicationLifecycle> {
        Box::new(self)
    }
}

#[async_trait]
impl ApplicationLifecycle for TestApplicationLifecycle {
    async fn on_start(
        &mut self,
        _ctx: &mut ApplicationContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Application Started...");

        Ok(())
    }

    async fn on_shutdown(&mut self, ctx: &ShutdownContext) {
        info!("Shutdown reason: {:?}", ctx.reason);
    }
}

impl Ordered for TestApplicationLifecycle {
    fn order(&self) -> i32 {
        0
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
