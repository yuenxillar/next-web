use std::any::Any;

use next_web_core::anys::any_value::AnyValue;

pub trait Authentication
where
    Self: Send + Sync,
{
    fn as_any(&self) -> &dyn Any;

    fn authentication_type(&self) -> &'static str;

    fn get_authorities(&self) -> Vec<String> {
        self.authorities()
    }

    fn get_credentials(&self) -> Option<String> {
        None
    }

    fn get_details(&self) -> Option<String> {
        self.get_details_ref().map(ToString::to_string)
    }

    fn get_details_value(&self) -> Option<AnyValue> {
        self.get_details_ref().cloned()
    }

    fn get_details_ref(&self) -> Option<&AnyValue> {
        None
    }

    fn get_principal(&self) -> Option<String> {
        None
    }

    fn get_name(&self) -> String {
        self.get_principal().unwrap_or_default()
    }

    fn is_authenticated(&self) -> bool;

    fn set_authenticated(&mut self, _is_authenticated: bool) -> Result<(), &'static str> {
        Ok(())
    }

    fn authorities(&self) -> Vec<String> {
        Vec::new()
    }

    fn is_anonymous(&self) -> bool {
        false
    }

    fn is_remember_me(&self) -> bool {
        false
    }
}
