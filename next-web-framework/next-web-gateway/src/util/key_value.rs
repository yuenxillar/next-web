#[derive(Debug, Clone)]
pub struct KeyValue<T: Clone> {
    pub k: String,
    pub v: T,
}

impl Into<KeyValue<String>> for Vec<&str> {
    fn into(self) -> KeyValue<String> {
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
