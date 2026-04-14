use std::{error::Error, sync::Arc};

use next_web_core::{
    async_trait, error::BoxError, traits::config::auto_configuration::AutoConfiguration,
    ApplicationContext,
};
use rudi_dev::singleton;
use tracing::warn;

use crate::{
    autoconfigure::mqtt_properties::MQTTClientProperties,
    interceptor::message_interceptor::MessageInterceptor,
    poll_error_handler::{DefaultMQTTPollErrorHandler, MQTTPollErrorHandler},
    service::default_mqtt_service::DefaultMQTTService,
    topic_listener::TopicListener,
    topic_router::{TopicRouteMatch, TopicRouter},
};

/// Auto-configuration for  Websocket.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct MQTTAutoConfiguration {
    /// Websocket properties.
    #[autowired(name = "mQTTClientProperties")]
    pub mqtt_client_properties: MQTTClientProperties,
}

impl MQTTAutoConfiguration {
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for MQTTAutoConfiguration {
    async fn configuration(&mut self, ctx: &mut ApplicationContext) -> Result<(), Box<dyn Error>> {
        let mqtt_properties = self.mqtt_client_properties.clone();

        let listeners = ctx.resolve_by_type::<Arc<dyn TopicListener>>();

        if listeners.is_empty() {
            warn!("No MQTT topic listener found");

            return Ok(());
        }

        let mut topic_router = TopicRouter::default();

        for listener in listeners
            .into_iter()
            .filter(|listener| !listener.topic().is_empty())
        {
            let topic = listener.topic();
            if topic.contains('#') || topic.contains('+') {
                topic_router
                    .match_
                    .push(TopicRouteMatch::new(topic, listener).map_err(std::io::Error::other)?)
            } else {
                topic_router.exact.insert(topic.into(), listener);
            }
        }

        let mut poll_error_handlers = ctx.resolve_by_type::<Arc<dyn MQTTPollErrorHandler>>();
        let poll_error_handler = if poll_error_handlers.len() > 1 {
            warn!("Multiple MQTT poll error handlers found, using the first one");
            poll_error_handlers.remove(0)
        } else if let Some(handler) = poll_error_handlers.pop() {
            handler
        } else {
            Arc::new(DefaultMQTTPollErrorHandler::default())
        };

        let interceptors = ctx.resolve_by_type::<Box<dyn MessageInterceptor>>();
        let mqtt_service = DefaultMQTTService::new(
            mqtt_properties,
            topic_router,
            interceptors,
            poll_error_handler,
        )?;
        ctx.insert_singleton_with_default_name(mqtt_service);

        Ok(())
    }
}
