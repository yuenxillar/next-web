use std::sync::Arc;

use crate::common::group_name::GroupName;

pub trait Group
where
    Self: Send + Sync,
{
    fn group(&self) -> GroupName {
        GroupName {
            name: std::env::var("CARGO_PKG_NAME").unwrap_or(String::from("unknown")),
            type_name: std::any::type_name::<Self>(),
        }
    }
}


impl<T: Group + ?Sized> Group for Box<T> {
    fn group(&self) -> GroupName {
        (**self).group()
    }
}


impl<T: Group + ?Sized> Group for Arc<T> {
    fn group(&self) -> GroupName {
        (**self).group()
    }
}

