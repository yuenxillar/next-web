use std::collections::HashMap;

use crate::anys::any_value::AnyValue;

#[derive(Debug, Default, Clone)]
pub struct MessageHeaders {
    headers: HashMap<String, AnyValue>,
}

impl MessageHeaders {
    pub fn get(&self, header: &str) -> Option<&AnyValue> {
        self.headers.get(header)
    }

    pub fn raw_headers(&self) -> HashMap<String, AnyValue> {
        self.headers.clone()
    }
}

impl From<HashMap<String, AnyValue>> for MessageHeaders {
    fn from(headers: HashMap<String, AnyValue>) -> Self {
        Self { headers }
    }
}
