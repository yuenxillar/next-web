use crate::autoregister::auto_register::AutoRegister;
use std::sync::Arc;

pub struct ApplicationDefaultRegisterSingle(Vec<Arc<dyn AutoRegister>>);

impl ApplicationDefaultRegisterSingle {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push<T>(&mut self)
    where
        T: AutoRegister + Default,
    {
        self.0.push(Arc::new(T::default()));
    }

    pub fn register_all(&mut self, ctx: &mut rudi::Context) {
        for register in self.0.iter() {
            // If panic early exit.
            register.register(ctx).unwrap();
        }
    }
}
