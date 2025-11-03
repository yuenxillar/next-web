use next_web_core::anys::any_value::AnyValue;

pub trait ObjectPostProcessor<T>
where
    T: Send + Sync,
    Self: Send + Sync,
{
    fn post_process(&self, object: AnyValue) -> Option<AnyValue>;
}
