use crate::web::csrf::CsrfToken;




pub trait DeferredCsrfToken {
    
    fn get_token(&self) -> &dyn CsrfToken;

    fn is_generated(&self) -> bool;
}