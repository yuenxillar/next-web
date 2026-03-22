use std::time::Duration;

use crate::traits::message::Message;

pub trait MessageChannel<T>
where
    T: Send,
{
    fn send(&self, message: Box<dyn Message<T>>, timeout: Duration);
}
