use next_web_core::traits::required::Required;

use crate::{
    core::session::{mgt::{
        default_session_context::DefaultSessionContext, session_context::SessionContext,
    }, SessionId},
    web::session::mgt::web_session_context::WebSessionContext,
};

#[derive(Clone)]
pub struct DefaultWebSessionContext
where
    Self: Required<DefaultSessionContext>,
{
    default_session_context: DefaultSessionContext,
}

impl DefaultWebSessionContext {
    const REQUEST: &str = stringify!(format!("{}.REQUEST", std::any::type_name::<Self>()));
    const RESPONSE: &str = stringify!(format!("{}.RESPONSE", std::any::type_name::<Self>()));
}
impl SessionContext for DefaultWebSessionContext {
    fn set_host(&mut self, host: &str) {
        self.default_session_context.set_host(host)
    }
    
    fn get_host(&self) -> Option<&str> {
        self.default_session_context.get_host()
    }
    
    fn set_session_id(&mut self, session_id: SessionId) {
        self.default_session_context.set_session_id(session_id)
    }
 
    fn get_session_id(&self) -> Option<&SessionId> {
        self.default_session_context.get_session_id()
    }
}

impl WebSessionContext for DefaultWebSessionContext {}

impl Required<DefaultSessionContext> for DefaultWebSessionContext {
    fn get_object(&self) -> &DefaultSessionContext {
        & self.default_session_context
    }

    fn get_mut_object(&mut self) -> &mut DefaultSessionContext {
        &mut self.default_session_context
    }
}

impl Default for DefaultWebSessionContext {
    fn default() -> Self {
        Self {
            default_session_context: Default::default(),
        }
    }
}
