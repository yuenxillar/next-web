use next_web::{
    Application, NextWebApplication,
    core::{
        ApplicationContext, Ordered, async_trait,
        traits::application::application_lifecycle::{ApplicationLifecycle, ShutdownContext},
    },
    macros::bind::singleton,
};
use tracing::info;

#[derive(Default)]
pub struct TestApplication;

impl Application for TestApplication {}

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
        _ctx: &mut dyn ApplicationContext,
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
    NextWebApplication::<TestApplication>::default().run().await
}
