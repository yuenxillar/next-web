use std::collections::HashMap;

use next_web_core::Required;

use crate::core::{session::SessionId, util::map_context::MapContext, util::object::Object};

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
            .map(Object::to_value)
            .unwrap_or_default()
    }

    fn put_all(&mut self, values: Vec<(String, Object)>) {
        self.map_context.extend(values);
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::Required;

    use super::DefaultSessionContext;
    use crate::core::{
        session::{mgt::session_context::SessionContext, SessionId},
        util::object::Object,
    };

    #[test]
    fn the_host_and_the_session_id_are_readable() {
        let mut context = DefaultSessionContext::default();
        context.set_host("  example.org  ");
        context.set_session_id(SessionId::String("the-session".to_string()));

        assert_eq!(context.get_host(), Some("example.org"));
        assert_eq!(
            context.get_session_id().map(ToString::to_string),
            Some("the-session".to_string())
        );
    }

    #[test]
    fn an_empty_host_is_not_recorded() {
        let mut context = DefaultSessionContext::default();
        context.set_host("   ");

        assert_eq!(context.get_host(), None);
    }

    #[test]
    fn the_values_of_a_context_are_merged_into_it() {
        let mut context = DefaultSessionContext::default();
        context.put_all(vec![
            ("first".to_string(), Object::Str("a".to_string())),
            ("second".to_string(), Object::Int(2)),
        ]);

        assert_eq!(context.get_object().get("first").and_then(Object::as_str), Some("a"));
        assert_eq!(context.get_object().get("second").and_then(Object::as_int), Some(2));
    }
}
