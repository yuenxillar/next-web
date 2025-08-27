use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct GroupName {
    pub(crate) name: String,
    pub(crate) type_name: &'static str,
}

impl GroupName {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn type_name(&self) -> &str {
        self.type_name
    }
}

impl Display for GroupName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, type_name: {}", self.name, self.type_name)
    }
}
