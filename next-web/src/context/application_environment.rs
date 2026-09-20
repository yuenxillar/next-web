use next_web_core::env::{
    ConfigurableEnvironment, ConfigurablePropertyResolver, Environment, PropertyResolver,
};

#[derive(Default)]
pub struct ApplicationEnvironment {}

impl ConfigurableEnvironment for ApplicationEnvironment {
    fn add_active_profile(&mut self, profile: &str) {
        todo!()
    }

    fn system_environment(&self) -> std::collections::HashMap<String, String> {
        todo!()
    }

    fn set_active_profiles(&mut self, profiles: &[&str]) {
        todo!()
    }

    fn set_default_profiles(&mut self, profiles: &[&str]) {
        todo!()
    }

    fn system_properties(&self) -> std::collections::HashMap<String, String> {
        todo!()
    }
}

impl ConfigurablePropertyResolver for ApplicationEnvironment {}

impl Environment for ApplicationEnvironment {
    fn accepts_profiles(&self, profiles: &dyn next_web_core::env::Profiles) -> bool {
        todo!()
    }

    fn active_profiles(&self) -> &[String] {
        todo!()
    }

    fn default_profiles(&self) -> &[String] {
        todo!()
    }

    fn matches_profiles(
        &self,
        profile_expressions: &[&str],
    ) -> Result<bool, next_web_core::env::EnvError> {
        todo!()
    }
}

impl PropertyResolver for ApplicationEnvironment {
    fn contains_property(&self, key: &str) -> bool {
        todo!()
    }

    fn get_property(&self, key: &str) -> Option<String> {
        todo!()
    }

    fn get_property_or_default(&self, key: &str, default_value: &str) -> &str {
        todo!()
    }

    fn get_required_property(
        &self,
        key: &str,
    ) -> Result<String, next_web_core::error::IllegalError> {
        todo!()
    }

    fn resolve_placeholders(&self, text: &str) -> String {
        todo!()
    }

    fn resolve_required_placeholders(
        &self,
        text: &str,
    ) -> Result<String, next_web_core::error::IllegalError> {
        todo!()
    }
}
