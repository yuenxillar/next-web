use std::collections::HashMap;

/// Simple HTML template engine for constructing login page HTML.
pub struct HtmlTemplates {
    template: String,
    values: HashMap<String, String>,
}

impl HtmlTemplates {
    pub fn from_template(template: &str) -> Self {
        Self {
            template: template.to_string(),
            values: HashMap::new(),
        }
    }

    pub fn with_value(mut self, key: &str, value: &str) -> Self {
        self.values.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_raw_html(mut self, key: &str, value: &str) -> Self {
        self.values.insert(key.to_string(), value.to_string());
        self
    }

    pub fn render(&self) -> String {
        let mut result = self.template.clone();
        for (key, value) in &self.values {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}
