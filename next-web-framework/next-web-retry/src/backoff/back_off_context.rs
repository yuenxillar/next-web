use next_web_core::util::any_map::AnyValue;


pub trait BackOffContext
where
    Self: Send + Sync
{
    fn get_value(&self) -> Option<&AnyValue>;
}

impl BackOffContext for AnyValue {
    fn get_value(&self) -> Option<&AnyValue> {
        Some(self)
    }
}