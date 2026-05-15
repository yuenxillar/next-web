use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct DirContextOperations {
    dn: String,
    attributes: HashMap<String, Vec<String>>,
}

impl DirContextOperations {
    pub fn new(dn: impl Into<String>) -> Self {
        Self {
            dn: dn.into(),
            attributes: HashMap::new(),
        }
    }

    pub fn dn(&self) -> &str {
        &self.dn
    }

    pub fn set_attribute(
        &mut self,
        name: impl Into<String>,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) {
        self.attributes
            .insert(name.into(), values.into_iter().map(Into::into).collect());
    }

    pub fn attribute(&self, name: &str) -> Option<&[String]> {
        self.attributes.get(name).map(Vec::as_slice)
    }

    pub fn attribute_first(&self, name: &str) -> Option<&str> {
        self.attributes
            .get(name)
            .and_then(|values| values.first())
            .map(String::as_str)
    }
}
