#[derive(PartialEq, Eq, Clone)]
pub enum BuilderRule {
    MVC,
    DDD,
}

impl Default for BuilderRule {
    fn default() -> Self {
        BuilderRule::DDD
    }
}
