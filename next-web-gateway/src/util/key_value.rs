//! A small key/value pair used by the filter constructors.

/// A borrowed-free `name`/`value` pair.
#[derive(Debug, Clone)]
pub struct KeyValue<V: Clone, K = Box<str>> {
    /// Name part.
    pub k: K,
    /// Value part.
    pub v: V,
}

impl From<Vec<&str>> for KeyValue<String> {
    fn from(value: Vec<&str>) -> Self {
        if value.len() == 2 {
            KeyValue {
                k: value[0].into(),
                v: value[1].to_owned(),
            }
        } else {
            KeyValue {
                k: "".into(),
                v: "".into(),
            }
        }
    }
}

impl From<Vec<&str>> for KeyValue<String, String> {
    fn from(value: Vec<&str>) -> Self {
        if value.len() == 2 {
            KeyValue {
                k: value[0].to_owned(),
                v: value[1].to_owned(),
            }
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
            k: value.0.to_owned(),
            v: value.1.to_owned(),
        }
    }
}

impl From<Vec<String>> for KeyValue<Box<str>> {
    fn from(value: Vec<String>) -> Self {
        let first = value.first().map(String::as_str).unwrap_or_default();
        let second = value.get(1).map(String::as_str).unwrap_or_default();

        KeyValue::from((first, second))
    }
}
