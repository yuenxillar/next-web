use std::{
    any::{Any, TypeId},
    fmt::{self, Display},
    hash::{Hash, Hasher},
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

    pub fn as_object<T: AnyObject>(&self) -> Option<&T> {
        match self {
            Object::Obj(obj) => (obj as &dyn Any).downcast_ref::<T>(),
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
