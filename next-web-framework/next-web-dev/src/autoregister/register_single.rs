use next_web_core::{
    autoregister::auto_register::AutoRegister,
    context::{application_context::ApplicationContext, properties::ApplicationProperties},
};

use crate::autoregister::message_source_service_autoregister::MessageSourceServiceAutoRegister;

use super::job_scheduler_autoregister::JobSchedulerAutoRegister;

pub struct ApplicationDefaultRegisterContainer {
    registers: Vec<Box<dyn AutoRegister>>,
}

impl ApplicationDefaultRegisterContainer {
 

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
        self.push::<MessageSourceServiceAutoRegister>();

        for register in self.registers.iter() {
            register.register(ctx, properties).await.unwrap();
        }
    }
}

impl Default for ApplicationDefaultRegisterContainer {
    fn default() -> Self {
        Self {
            registers: Vec::new(),
        }
    }
}
 