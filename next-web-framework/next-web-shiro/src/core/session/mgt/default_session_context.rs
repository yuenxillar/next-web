use std::collections::HashMap;

use next_web_core::traits::required::Required;

use crate::core::{object::Object, session::SessionId, util::map_context::MapContext};

use super::session_context::SessionContext;

#[derive(Clone)]
pub struct DefaultSessionContext
where
    Self: Required<MapContext>,
{
    map_context: MapContext,
}

impl DefaultSessionContext {
    const HOST: &str = stringify!(format!("{}.HOST", std::any::type_name::<Self>()));

    const SESSION_ID: &str = stringify!(format!("{}.SESSION_ID", std::any::type_name::<Self>()));

    pub fn new(map: HashMap<String, Object>) -> Self {
        Self {
            map_context: MapContext::new(map),
        }
    }
}

impl Default for DefaultSessionContext {
    fn default() -> Self {
        Self {
            map_context: Default::default(),
        }
    }
}
impl Required<MapContext> for DefaultSessionContext {
    fn get_object(&self) -> &MapContext {
        &self.map_context
    }

    fn get_mut_object(&mut self) -> &mut MapContext {
        &mut self.map_context
    }
}

impl SessionContext for DefaultSessionContext {
    fn set_host(&mut self, host: &str) {
        let host = host.trim();
        if !host.is_empty() {
            self.map_context
                .insert(Self::HOST.to_string(), Object::Str(host.to_string()));
        }
    }

    fn get_host(&self) -> Option<&str> {
        self.map_context
            .get(Self::HOST)
            .map(Object::as_str)
            .unwrap_or_default()
    }

    fn set_session_id(&mut self, session_id: SessionId) {
        self.map_context.insert(
            Self::SESSION_ID.to_string(),
            Object::Obj(Box::new(session_id)),
        );
    }

    fn get_session_id(&self) -> Option<&SessionId> {
        self.map_context
            .get(Self::SESSION_ID)
            .map(Object::as_object)
            .unwrap_or_default()
    }
}
