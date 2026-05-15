use std::collections::HashMap;

pub trait RequiredAuthoritiesRepository: Send + Sync {
    fn find_required_authorities(&self, username: &str) -> Vec<String>;
}

#[derive(Clone, Debug, Default)]
pub struct MapRequiredAuthoritiesRepository {
    authorities: HashMap<String, Vec<String>>,
}

impl MapRequiredAuthoritiesRepository {
    pub fn new(authorities: HashMap<String, Vec<String>>) -> Self {
        Self { authorities }
    }

    pub fn set_required_authorities(&mut self, username: impl Into<String>, authorities: Vec<String>) {
        self.authorities.insert(username.into(), authorities);
    }
}

impl RequiredAuthoritiesRepository for MapRequiredAuthoritiesRepository {
    fn find_required_authorities(&self, username: &str) -> Vec<String> {
        self.authorities.get(username).cloned().unwrap_or_default()
    }
}
