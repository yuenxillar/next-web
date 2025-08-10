use next_web_core::DynClone;

use crate::{observation::observation::Context, util::key_values::KeyValues};

pub trait ObservationConvention<T>: Send + Sync
where
    Self: DynClone,
{
    fn low_cardinality_key_values(&self) -> KeyValues<()> {
        KeyValues::empty()
    }

    fn high_cardinality_key_values(&self) -> KeyValues<()> {
        KeyValues::empty()
    }

    fn supports_context(&self, context: &dyn Context) -> bool;

    fn name(&self) -> Option<&str> {
        return None;
    }

    fn contextual_name(&self, context: &T) -> Option<&str> {
        return None;
    }
}

next_web_core::clone_trait_object!(<T>  ObservationConvention<T> where T: Send + Sync + Clone);
