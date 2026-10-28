use std::{
    any::{Any, TypeId},
    fmt::{self, Display},
    hash::Hasher,
};

use next_web_core::{clone_box, DynClone};

// 动态 Hasher 包装器
pub struct DynHasher<'a>(&'a mut dyn Hasher);

impl<'a> Hasher for DynHasher<'a> {
    fn finish(&self) -> u64 {
        self.0.finish()
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes);
    }
}

pub trait AnyObject: Any + DynClone + Send + Sync {
    // fn equals(&self, other: &dyn AnyObject) -> bool;
    fn object_type_id(&self) -> TypeId;
    // fn hash_object(&self, hasher: &mut dyn Hasher);
}

impl<T> AnyObject for T
where
    T: Any + DynClone,
    T: Send + Sync,
{
    // fn equals(&self, other: &dyn AnyObject) -> bool {
    //     if let Some(other_value) = (other as &dyn Any).downcast_ref::<T>() {
    //         self == other_value
    //     } else {
    //         false
    //     }
    // }

    fn object_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    // fn hash_object(&self, hasher: &mut dyn Hasher) {
    //     let mut wrapper = DynHasher(hasher);
    //     self.hash(&mut wrapper);
    // }
}

// impl PartialEq for Box<dyn AnyObject> {
//     fn eq(&self, other: &Self) -> bool {
//         self.equals(&**other)
//     }
// }

// impl Eq for Box<dyn AnyObject> {}

// impl Hash for Box<dyn AnyObject> {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.object_type_id().hash(state);
//         self.hash_object(state);
//     }
// }

impl Clone for Box<dyn AnyObject> {
    fn clone(&self) -> Box<dyn AnyObject> {
        clone_box(&**self)
    }
}

#[derive(Clone)]
pub enum Object {
    Str(String),
    Int(i64),
    Bool(bool),
    List(Vec<Object>),
    Obj(Box<dyn AnyObject>),
    ListStr(Vec<String>),
    Null,
}

impl Object {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Object::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Object::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Object::Bool(bool) => Some(*bool),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<Object>> {
        match self {
            Object::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn to_value<T: AnyObject>(&self) -> Option<&T> {
        match self {
            // The value behind the box is the one that was stored, so the box is
            // opened before it is downcast; downcasting the box itself would only
            // ever match `Box<dyn AnyObject>`.
            Object::Obj(obj) => (obj.as_ref() as &dyn Any).downcast_ref::<T>(),
            _ => None,
        }
    }

    pub fn as_list_str(&self) -> Option<Vec<&str>> {
        match self {
            Object::ListStr(obj) => Some(obj.iter().map(String::as_str).collect()),
            _ => None,
        }
    }

    pub fn into_any_clone(self) -> Option<Box<dyn AnyObject>> {
        match self {
            Object::Obj(obj) => Some(obj),
            _ => None,
        }
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Object::Str(s) => write!(f, "{}", s),
            Object::Int(i) => write!(f, "{}", i),
            Object::Bool(b) => write!(f, "{}", b),
            Object::Obj(obj) => write!(f, "{:?}", obj),
            Object::List(_objects) => write!(f, "{}", "List[Object]"),
            Object::ListStr(list_str) => write!(f, "{:?}", list_str),
            Object::Null => write!(f, "{}", "Null"),
        }
    }
}

impl fmt::Debug for dyn AnyObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use next_web_core::DynClone;

    use super::Object;

    /// A value a caller stores in an object and reads back.
    #[derive(Clone, Debug, PartialEq)]
    struct Account {
        name: String,
    }

    #[test]
    fn a_string_and_a_number_are_readable_as_themselves() {
        assert_eq!(Object::Str("root".to_string()).as_str(), Some("root"));
        assert_eq!(Object::Int(42).as_int(), Some(42));
        assert_eq!(Object::Bool(true).as_bool(), Some(true));
        assert_eq!(Object::Str("root".to_string()).as_int(), None);
        assert_eq!(Object::Null.as_str(), None);
    }

    #[test]
    fn a_list_of_objects_is_readable() {
        let list = Object::List(vec![Object::Int(1), Object::Int(2)]);

        assert_eq!(list.as_list().map(|list| list.len()), Some(2));
        assert!(Object::Null.as_list().is_none());
    }

    #[test]
    fn a_list_of_strings_is_readable() {
        let list = Object::ListStr(vec!["admin".to_string(), "user".to_string()]);

        assert_eq!(list.as_list_str(), Some(vec!["admin", "user"]));
        assert_eq!(Object::Null.as_list_str(), None);
    }

    #[test]
    fn an_object_round_trips_through_a_value() {
        let object = Object::Obj(Box::new(Account {
            name: "root".to_string(),
        }));

        let account = object
            .to_value::<Account>()
            .expect("the value behind the object is the one that was stored");
        assert_eq!(account.name, "root");
        assert!(object.to_value::<String>().is_none());
    }

    #[test]
    fn an_arc_round_trips_through_a_value() {
        let value: Arc<dyn AccountTrait> = Arc::new(Account {
            name: "root".to_string(),
        });
        let object = Object::Obj(Box::new(value));

        let read = object
            .to_value::<Arc<dyn AccountTrait>>()
            .expect("the arc is the value that was stored");
        assert_eq!(read.name(), "root");
    }

    #[test]
    fn an_object_hands_back_the_value_it_holds() {
        let object = Object::Obj(Box::new(Account {
            name: "root".to_string(),
        }));

        let account = object
            .into_any_clone()
            .expect("the value is an object");
        assert_eq!(account.object_type_id(), std::any::TypeId::of::<Account>());
    }

    #[test]
    fn an_object_renders_as_its_value() {
        assert_eq!(Object::Str("root".to_string()).to_string(), "root");
        assert_eq!(Object::Int(42).to_string(), "42");
        assert_eq!(Object::Bool(false).to_string(), "false");
        assert_eq!(Object::Null.to_string(), "Null");
        assert_eq!(
            Object::ListStr(vec!["a".to_string(), "b".to_string()]).to_string(),
            "[\"a\", \"b\"]"
        );
    }

    /// The trait a value is stored behind.
    trait AccountTrait: Send + Sync + DynClone {
        fn name(&self) -> &str;
    }

    impl AccountTrait for Account {
        fn name(&self) -> &str {
            &self.name
        }
    }

    next_web_core::clone_trait_object!(AccountTrait);
}