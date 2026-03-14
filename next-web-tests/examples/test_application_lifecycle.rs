use next_web::{
    async_trait,
    traits::{
        application::application_lifecycle::{ApplicationLifecycle, ShutdownContext},
        ordered::Ordered,
    },
    ApplicationContext,
};

#[derive(Clone)]
#[allow(unused)]
struct TestApplicationLifecycle;

#[async_trait]
impl ApplicationLifecycle for TestApplicationLifecycle {
    async fn on_start(
        &mut self,
        _ctx: &mut ApplicationContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Application Started");

        Ok(())
    }

    async fn on_shutdown(&mut self, ctx: &ShutdownContext) {
        println!("Shutdown reason: {:?}", ctx.reason);
    }
}

impl Ordered for TestApplicationLifecycle {
    fn order(&self) -> i32 {
        0
    }
}

#[tokio::main]
async fn main() {}
