use next_web_core::{
    autoregister::auto_register::AutoRegister,
    context::{application_context::ApplicationContext, properties::ApplicationProperties},
};


#[cfg(feature = "enable-i18n")]
use crate::autoregister::message_source_service_autoregister::MessageSourceServiceAutoRegister;


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
 