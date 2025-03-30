use crate::autoregister::auto_register::AutoRegister;

pub struct ApplicationDefaultRegisterSingle {
    registers: Vec<Box<dyn AutoRegister + Send + Sync>>
}

impl ApplicationDefaultRegisterSingle {
    pub fn new() -> Self {
        Self { registers: Vec::new() }
    }

    pub fn push<T>(&mut self)
    where
        T: AutoRegister + Default + Send + Sync + 'static,
    {
        self.registers.push(Box::new(T::default()));
    }

    pub fn register_all(&mut self, ctx: &mut rudi::Context) {
        for register in self.registers.iter() {
            // If panic early exit.
            register.register(ctx).unwrap();
        }
    }
}
