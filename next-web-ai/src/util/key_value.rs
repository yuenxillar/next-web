pub(crate) const NONE_VALUE: &'static str = "None";

use next_web_core::DynClone;

pub trait KeyValue
where
    Self: DynClone,
    Self: Send + Sync,
{
    fn key(&self) -> &str;

    fn value(&self) -> &str;
}

next_web_core::clone_trait_object!(KeyValue);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoneKeyValue;

impl NoneKeyValue {
    pub fn of_immutable(key: impl Into<String>, value: impl Into<String>) -> ImmutableKeyValue {
        ImmutableKeyValue::new(key, value)
    }

    pub fn of_validated<T>(
        key: impl Into<String>,
        value: T,
        validator: impl Predicate<T>,
    ) -> ValidatedKeyValue
    where
        T: ToString,
    {
        ValidatedKeyValue::new(key, value, validator)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImmutableKeyValue {
    key: String,
    value: String,
}

impl ImmutableKeyValue {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl KeyValue for ImmutableKeyValue {
    fn key(&self) -> &str {
        &self.key
    }

    fn value(&self) -> &str {
        todo!()
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValidatedKeyValue {
    key: String,
    value: String,
}

impl ValidatedKeyValue {
    pub fn new<T>(key: impl Into<String>, value: T, validator: impl Predicate<T>) -> Self
    where
        T: ToString,
    {
        assert!(validator.test(&value));
        Self {
            key: key.into(),
            value: value.to_string(),
        }
    }
}

impl KeyValue for ValidatedKeyValue {
    fn key(&self) -> &str {
        &self.key
    }

    fn value(&self) -> &str {
        &self.value
    }
}

pub trait Predicate<T> {
    fn test(&self, t: &T) -> bool;
}

impl KeyValue for () {
    fn key(&self) -> &str {
        ""
    }

    fn value(&self) -> &str {
        ""
    }
}
