use crate::interface::singleton::Singleton;


pub trait Ordered {
    fn order(&self) -> i32;
}

impl<T> Ordered for T 
where 
T: Singleton
{
    fn order(&self) -> i32 {
        i32::MAX
    }
}