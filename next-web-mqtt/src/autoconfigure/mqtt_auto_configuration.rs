use next_web_context::ApplicationContextExt;
use std::{error::Error, sync::Arc};

use next_web_core::{ApplicationContext, Ordered, async_trait, config::AutoConfiguration};
use next_web_macros::singleton;
use tracing::warn;

use crate::{
    MqttClient,
    autoconfigure::mqtt_properties::MQTTClientProperties,
    interceptor::message_interceptor::MessageInterceptor,
    poll_error_handler::{DefaultMQTTPollErrorHandler, MQTTPollErrorHandler},
    topic_listener::TopicListener,
    topic_router::{TopicRouteMatch, TopicRouter},
};

/// Auto-configuration for the MQTT client.
///
/// The configuration collects the topic listeners, poll error handlers and
/// message interceptors of the application, turns the listeners into a
/// [`TopicRouter`] and registers a [`MqttClient`] singleton that owns the client
/// and its event loop.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct MQTTAutoConfiguration {
    /// MQTT client configuration properties.
    #[autowired]
    pub mqtt_client_properties: MQTTClientProperties,
}

impl MQTTAutoConfiguration {
    /// Erases the configuration into the trait object the bootstrap resolves.
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for MQTTAutoConfiguration {
    /// Builds the MQTT service out of the components of the context.
    ///
    /// The service is only registered when the application declares at least
    /// one topic listener, so an application that does not consume MQTT
    /// messages does not open a broker connection. As a consequence the
    /// auto-configuration does not provide the client to an application that
    /// only publishes messages.
    ///
    /// # Errors
    ///
    /// Returns an error when a wildcard subscription filter of a listener is
    /// malformed, or when the client cannot be built from the properties.
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
        if ctx.contains_singleton_with_default_name::<MqttClient>() {
            return Ok(());
        }

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
                topic_router
                    .exact
                    .entry(topic.into())
                    .or_default()
                    .push(listener);
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
        let mqtt_client = MqttClient::new(
            mqtt_properties,
            topic_router,
            interceptors,
            poll_error_handler,
        )?;
        ctx.insert_singleton_with_default_name(mqtt_client);

        Ok(())
    }
}

/// The configuration is applied in the default order of [`Ordered`].
impl Ordered for MQTTAutoConfiguration {}
