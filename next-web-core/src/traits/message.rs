use dyn_clone::{clone_trait_object, DynClone};

use crate::messaging::message_headers::MessageHeaders;

pub trait Message<T>
where
    Self: Send + Sync,
    Self: DynClone,
{
    fn get_payload(&self) -> Option<&T>;

    fn get_own_payload(&mut self) -> Option<T>;

    fn get_headers(&self) -> &MessageHeaders;
}

clone_trait_object!(<T> Message<T>);
