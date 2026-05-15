#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ObjectIdentity {
    object_type: String,
    identifier: String,
}

impl ObjectIdentity {
    pub fn new(object_type: impl Into<String>, identifier: impl ToString) -> Self {
        let object_type = object_type.into();
        assert!(!object_type.trim().is_empty(), "Type required");
        let identifier = identifier.to_string();
        assert!(!identifier.trim().is_empty(), "identifier required");
        Self {
            object_type,
            identifier,
        }
    }

    pub fn object_type(&self) -> &str {
        &self.object_type
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }
}
