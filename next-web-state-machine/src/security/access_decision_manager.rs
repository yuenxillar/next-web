use next_web_core::{anys::any_value::AnyValue, error::BoxError};

pub trait AccessDecisionManager
where
    Self: Send + Sync,
{
    fn decide(
        &self,
        authentication: &AnyValue,
        value: &AnyValue,
        config_attributes: Vec<Box<dyn ConfigAttribute>>,
    ) -> Result<(), BoxError>;

    fn supports(&self, attribute: &dyn ConfigAttribute) -> bool;
}

pub trait ConfigAttribute {
    fn get_attribute(&self) -> Option<&str>;
}
