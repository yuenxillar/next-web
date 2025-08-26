#[derive(Debug, Clone)]
pub struct KeyValue<V: Clone, K = Box<str>> {
    pub k: K,
    pub v: V,
}

impl Into<KeyValue<String>> for Vec<&str> {
    fn into(self) -> KeyValue<String> {
        if self.len() == 2 {
            let k = self[0].into();
            let v = self[1].to_string();
            KeyValue { k, v }
        } else {
            KeyValue {
                k: "".into(),
                v: "".into(),
            }
        }
    }
}

impl Into<KeyValue<String, String>> for Vec<&str> {
    fn into(self) -> KeyValue<String, String> {
        if self.len() == 2 {
            let k = self[0].to_string();
            let v = self[1].to_string();
            KeyValue { k, v }
        } else {
            KeyValue {
                k: "".into(),
                v: "".into(),
            }
        }
    }
}

impl From<(&str, &str)> for KeyValue<Box<str>, Box<str>> {
    fn from(value: (&str, &str)) -> Self {
        Self {
            k: Box::from(value.0),
            v: Box::from(value.1),
        }
    }
}

impl From<(&str, &str)> for KeyValue<String, String> {
    fn from(value: (&str, &str)) -> Self {
        Self {
            k: value.0.to_string(),
            v: value.1.to_string(),
        }
    }
}

impl Into<KeyValue<Box<str>>> for Vec<String> {
    fn into(self) -> KeyValue<Box<str>> {
        KeyValue::from((self[0].as_str(), self[1].as_str()))
    }
}
