use axum::Router;

use crate::ApplicationContext;



pub trait ApplyRouter: Send + Sync{
    
    fn router(&self, ctx: &mut ApplicationContext) -> Router;
}