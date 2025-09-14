use std::{
    any::Any,
    hash::{Hash, Hasher},
};

use next_web_core::DynClone;

pub mod back_off_interrupted_error;
pub mod retry_error;

pub trait AnyError
where
    Self: Send + Sync,
    Self: std::error::Error + DynClone,
{
    fn to_boxed(&self) -> Box<dyn AnyError>;

    fn as_any(&self) -> &dyn Any;

    fn hash_dyn(&self, state: &mut dyn Hasher);
}

impl<T> AnyError for T
where
    T: std::error::Error + Clone,
    T: Send + Sync + 'static,
    T: Hash,
{
    fn to_boxed(&self) -> Box<dyn AnyError> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn hash_dyn(&self, state: &mut dyn Hasher) {
        self.hash(&mut HasherWrapper(state));
    }
}

struct HasherWrapper<'a>(&'a mut dyn Hasher);

impl<'a> Hasher for HasherWrapper<'a> {
    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes);
    }

    fn finish(&self) -> u64 {
        self.0.finish()
    }
}

impl PartialEq for Box<dyn AnyError> {
    fn eq(&self, other: &Self) -> bool {
        if self.as_any().type_id() != other.as_any().type_id() {
            return false;
        }
        std::ptr::eq(self.as_ref(), other.as_ref())
    }
}

impl Eq for Box<dyn AnyError> {}

impl Hash for Box<dyn AnyError> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash_dyn(state);
    }
}

next_web_core::clone_trait_object!(AnyError where Self: Send + Sync);
