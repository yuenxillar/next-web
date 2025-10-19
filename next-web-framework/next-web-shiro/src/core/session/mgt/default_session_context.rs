use super::session_context::SessionContext;



pub struct DefaultSessionContext {}


impl Default for DefaultSessionContext {
    fn default() -> Self {
        Self {  }
    }
}
impl SessionContext for DefaultSessionContext {

    fn set_host(&mut self, host: &str) {
        
    }
}