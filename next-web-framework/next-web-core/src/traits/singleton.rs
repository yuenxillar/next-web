use std::sync::Arc;

use crate::{traits::group::Group, util::singleton::SingletonUtil};

pub trait Singleton
where
    Self: Send + Sync,
    Self: Group,
{
    fn singleton_name(&self) -> String
    {
        SingletonUtil::name::<Self>()
    }
}

impl<T: ?Sized + Singleton + Group> Singleton for Box<T> {}
impl<T: ?Sized + Singleton + Group> Singleton for Arc<T> {}