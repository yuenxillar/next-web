use crate::traits::ordered::Ordered;

pub trait PropertiesPostProcessor
where
    Self: dyn_clone::DynClone,
    Self: Send + Sync,
    Self: Ordered,
{
    fn post_process_properties(&mut self, mapping: Option<&mut serde_yaml::Value>);
}

dyn_clone::clone_trait_object!(PropertiesPostProcessor);
