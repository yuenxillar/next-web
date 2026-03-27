use std::pin::Pin;

use crate::ApplicationContext;

pub trait FromContext: Send {
    fn from_ctx<'a>(
        ctx: &'a mut ApplicationContext,
    ) -> Pin<Box<dyn Future<Output = Self> + Send + 'a>>;
}
