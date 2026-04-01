use crate::{
    config::{
        default_ws_handler_registration::DefaultWebSocketHandlerRegistration,
        ws_handler_registration::WebSocketHandlerRegistration,
        ws_handler_registry::WebSocketHandlerRegistry,
    },
    server::support::ws_handler_mapping::WebSocketHandlerMapping,
    ws_handler::WebSocketHandler,
};

pub struct DefaultWebSocketHandlerRegistry {
    order: i32,
    pub(crate) registrations: Vec<DefaultWebSocketHandlerRegistration>,
}
impl DefaultWebSocketHandlerRegistry {
    pub fn order(&self) -> i32 {
        self.order
    }

    pub fn set_order(&mut self, order: i32) {
        self.order = order;
    }

    pub fn handler_mapping(&mut self) -> WebSocketHandlerMapping {
        let mut hander_mapping = WebSocketHandlerMapping::default();

        for registration in self.registrations.iter_mut() {
            registration
                .mappings()
                .into_iter()
                .for_each(|(paths, handler)| {
                    paths
                        .into_iter()
                        .for_each(|path| hander_mapping.insert(path.as_str(), handler.clone()));
                });
        }

        hander_mapping
    }
}

impl WebSocketHandlerRegistry for DefaultWebSocketHandlerRegistry {
    fn add_handler(
        &mut self,
        ws_handler: std::sync::Arc<dyn WebSocketHandler>,
        paths: Vec<String>,
    ) -> &mut dyn WebSocketHandlerRegistration {
        let mut registration = DefaultWebSocketHandlerRegistration::default();
        registration.add_handler(ws_handler, paths);

        self.registrations.push(registration);
        self.registrations.last_mut().unwrap()
    }
}

impl Default for DefaultWebSocketHandlerRegistry {
    fn default() -> Self {
        Self {
            order: 1,
            registrations: Vec::with_capacity(4),
        }
    }
}
