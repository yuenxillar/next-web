use std::any::Any;

use next_web_core::anys::any_value::AnyValue;


pub trait BackOffContext
where
    Self: Send + Sync,
    Self: Any,
{
    fn get_value(&self) -> Option<&AnyValue>;
}

impl BackOffContext for AnyValue {
    fn get_value(&self) -> Option<&AnyValue> {
        Some(self)
    }
}