use crate::{traits::group::Group, util::singleton::SingletonUtil};

pub trait Singleton
where
    Self: Send + Sync,
{
    fn singleton_name(&self) -> String
    {
        SingletonUtil::name::<Self>()
    }
}

impl<T: ?Sized + Singleton> Group for T {}