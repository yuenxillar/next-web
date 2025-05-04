use next_web_core::{
    autoregister::auto_register::AutoRegister,
    context::{
        application_context::ApplicationContext,
        properties::ApplicationProperties,
    },
};

use super::job_scheduler_autoregister::JobSchedulerAutoRegister;

pub struct ApplicationDefaultRegisterContainer {
    registers: Vec<Box<dyn AutoRegister + Send + Sync>>,
}

impl ApplicationDefaultRegisterContainer {
    pub fn new() -> Self {
        Self {
            registers: Vec::new(),
        }
    }

    fn push<T>(&mut self)
    where
        T: AutoRegister + Default + 'static,
    {
        self.registers.push(Box::new(T::default()));
    }

    pub async fn register_all(
        &mut self,
        ctx: &mut ApplicationContext,
        properties: &ApplicationProperties,
    ) {
        self.push::<JobSchedulerAutoRegister>();

        for register in self.registers.iter() {
            register.register(ctx, properties).await.unwrap();
        }
    }
}
