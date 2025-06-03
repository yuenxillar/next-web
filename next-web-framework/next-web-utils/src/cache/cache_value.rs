use std::any::Any;


#[derive(Clone)]
pub enum CacheValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Float(String),
    Array(Vec<CacheValue>),
    // Object(Box<dyn BoxAny>),
    Null,
}

impl CacheValue {
    pub fn is_number(&self) -> bool {
        matches!(self, CacheValue::Number(_))
    }

    pub fn is_float(&self) -> bool {
        matches!(self, CacheValue::Float(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, CacheValue::String(_))
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, CacheValue::Boolean(_))
    }

    pub fn is_null(&self) -> bool {
        matches!(self, CacheValue::Null)
    }

    pub fn is_array(&self) -> bool {
        matches!(self, CacheValue::Array(_))
    }

    // pub fn is_object(&self) -> bool {
    //     matches!(self, CacheValue::Object(_))
    // }

    pub fn as_string(&self) -> Option<String> {
        if let CacheValue::String(s) = self {
            Some(s.to_owned())
        } else {
            None
        }
    }

    pub fn as_number(&self) -> Option<i64> {
        if let CacheValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        if let CacheValue::Boolean(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    pub fn as_array(&self) -> Option<&Vec<CacheValue>> {
        if let CacheValue::Array(a) = self {
            Some(a)
        } else {
            None
        }
    }

    // pub fn as_object<T: 'static>(&self) -> Option<T> {
    //     if let CacheValue::Object(obj) = self {
    //         if let Ok(o) = obj.as_any().downcast::<T>() {
    //             Some(*o)
    //         } else {
    //             None
    //         }
    //     } else {
    //         None
    //     }
    // }

    pub fn to_string(&self) -> String {
        match self {
            CacheValue::String(s) => s.clone(),
            CacheValue::Number(n) => n.to_string(),
            CacheValue::Boolean(b) => b.to_string(),
            CacheValue::Null => "null".to_string(),
            CacheValue::Array(a) => {
                let mut s = String::new();
                s.push('[');

                for v in a.iter() {
                    s.push_str(&v.to_string());
                    s.push(',');
                }
                s.push(']');
                s
            }
            _ => "".to_string(),
        }
    }
}

impl Into<CacheValue> for String {
    fn into(self) -> CacheValue {
        CacheValue::String(self)
    }
}

impl Into<CacheValue> for &str {
    fn into(self) -> CacheValue {
        CacheValue::String(self.to_string())
    }
}

impl Into<CacheValue> for i32 {
    fn into(self) -> CacheValue {
        CacheValue::Number(self as i64)
    }
}

impl Into<CacheValue> for i64 {
    fn into(self) -> CacheValue {
        CacheValue::Number(self)
    }
}

impl Into<CacheValue> for u32 {
    fn into(self) -> CacheValue {
        CacheValue::Number(self as i64)
    }
}

impl Into<CacheValue> for f32 {
    fn into(self) -> CacheValue {
        CacheValue::Float(self.to_string())
    }
}

impl Into<CacheValue> for bool {
    fn into(self) -> CacheValue {
        CacheValue::Boolean(self)
    }
}

impl Into<CacheValue> for Vec<CacheValue> {
    fn into(self) -> CacheValue {
        CacheValue::Array(self)
    }
}
