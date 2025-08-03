use crate::ApplicationContext;


pub trait FromContext: Send {
    
    fn from_ctx(ctx: &mut ApplicationContext) -> Self;
}