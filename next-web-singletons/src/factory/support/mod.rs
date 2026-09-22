mod default_listable_singleton_factory;
mod default_singleton_registry;

pub use self::default_listable_singleton_factory::DefaultListableSingletonFactory;
pub use self::default_singleton_registry::{DefaultSingletonRegistry, DynSingle, Key, Type};
