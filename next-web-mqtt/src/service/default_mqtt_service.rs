use std::{ops::Deref, sync::Arc};

use crate::{
    autoconfigure::mqtt_properties::{MQTTClientProperties, Topic},
    generate_client_id,
    interceptor::message_interceptor::MessageInterceptor,
    poll_error_handler::{
        is_connection_refused, MQTTPollErrorAction, MQTTPollErrorContext, MQTTPollErrorHandler,
    },
    service::mqtt_service::MQTTService,
    topic_router::TopicRouter,
};

use next_web_core::{
    async_trait, error::BoxError, impl_service, signal::APPLICATION_GRACEFUL_SHUTDOWN_SIGNAL,
};
use rumqttc::{
    AsyncClient, ClientError, ConnectReturnCode, ConnectionError, Event, EventLoop, MqttOptions,
    NetworkOptions, Packet, QoS, SubscribeFilter,
};
use tracing::{error, warn};

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 1883;
pub const DEFAULT_KEEP_ALIVE: u64 = 60;
pub const DEFAULT_CONNECT_TIMEOUT: u64 = 10;
pub const DEFAULT_CHANNEL_CAP: usize = 2048;

/// Default MQTT service.
///
/// This service is responsible for:
/// - configuring the MQTT client
/// - subscribing configured topics
/// - dispatching published messages to exact routes and wildcard routes
/// - invoking message interceptors before consumption
#[derive(Clone)]
pub struct DefaultMQTTService {
    /// MQTT client configuration properties.
    properties: Arc<MQTTClientProperties>,

    /// Async MQTT client instance.
    client: AsyncClient,
}

impl DefaultMQTTService {
    /// Creates a new `MQTTService`.
    pub fn new(
        properties: MQTTClientProperties,
        topic_router: TopicRouter,
        interceptor: Vec<Box<dyn MessageInterceptor>>,
        poll_error_handler: Arc<dyn MQTTPollErrorHandler>,
    ) -> Result<Self, BoxError> {
        let client =
            Self::build_client(&properties, topic_router, interceptor, poll_error_handler)?;

        let properties = Arc::new(properties);
        Ok(Self { properties, client })
    }

    /// Builds and configures the MQTT client, subscriptions and event loop.
    fn build_client(
        properties: &MQTTClientProperties,
        topic_router: TopicRouter,
        interceptors: Vec<Box<dyn MessageInterceptor>>,
        poll_error_handler: Arc<dyn MQTTPollErrorHandler>,
    ) -> Result<AsyncClient, BoxError> {
        let options = Self::build_options(properties);
        let cap = properties.cap().unwrap_or(DEFAULT_CHANNEL_CAP);
        let (client, mut eventloop) = AsyncClient::new(options, cap);

        Self::configure_network_options(properties, &mut eventloop);

        let topics = properties.topics();
        Self::subscribe_topics_try(&client, &topics)?;

        let max_retries = usize::from(properties.max_retries().unwrap_or(5));
        Self::spawn_event_loop(
            eventloop,
            client.clone(),
            topic_router,
            interceptors,
            poll_error_handler,
            topics,
            max_retries,
        );

        Ok(client)
    }

    fn build_options(properties: &MQTTClientProperties) -> MqttOptions {
        let mut options = MqttOptions::new(
            properties.client_id().unwrap_or(generate_client_id(false)),
            properties.host().unwrap_or(DEFAULT_HOST),
            properties.port().unwrap_or(DEFAULT_PORT),
        );

        options
            .set_keep_alive(std::time::Duration::from_secs(
                properties.keep_alive().unwrap_or(DEFAULT_KEEP_ALIVE),
            ))
            .set_clean_session(properties.clean_session().unwrap_or(true))
            .set_credentials(
                properties.username().unwrap_or_default(),
                properties.password().unwrap_or_default(),
            );

        options
    }

    fn configure_network_options(properties: &MQTTClientProperties, eventloop: &mut EventLoop) {
        let mut network_options = NetworkOptions::new();
        network_options.set_connection_timeout(
            properties
                .connect_timeout()
                .unwrap_or(DEFAULT_CONNECT_TIMEOUT),
        );
        eventloop.set_network_options(network_options);
    }

    fn spawn_event_loop(
        mut eventloop: EventLoop,
        reconnect_client: AsyncClient,
        topic_router: TopicRouter,
        interceptors: Vec<Box<dyn MessageInterceptor>>,
        poll_error_handler: Arc<dyn MQTTPollErrorHandler>,
        topics: Vec<Topic>,
        max_retries: usize,
    ) {
        tokio::spawn(async move {
            let mut retries_remaining = max_retries;

            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(packet))) => {
                        Self::handle_publish_event(
                            &topic_router,
                            &interceptors,
                            packet.topic.as_str(),
                            packet.payload.as_ref(),
                        )
                        .await;
                    }
                    Ok(Event::Incoming(Packet::ConnAck(ack))) => {
                        if ack.code == ConnectReturnCode::Success {
                            retries_remaining = max_retries;
                            Self::handle_successful_reconnect(&reconnect_client, &topics).await;
                        }
                    }
                    Err(error) => {
                        retries_remaining = Self::next_retries_remaining(retries_remaining, &error);

                        let action = poll_error_handler
                            .handle_poll_error(
                                &error,
                                MQTTPollErrorContext {
                                    max_retries,
                                    retries_remaining,
                                },
                            )
                            .await;

                        match action {
                            MQTTPollErrorAction::RetryAfter(delay) => {
                                tokio::time::sleep(delay).await;
                            }
                            MQTTPollErrorAction::Shutdown => {
                                Self::request_graceful_shutdown();
                                break;
                            }
                        }
                    }
                    _ => {
                        // Ignore other incoming or outgoing events for now.
                    }
                }
            }
        });
    }

    async fn handle_publish_event(
        topic_router: &TopicRouter,
        interceptors: &[Box<dyn MessageInterceptor>],
        topic: &str,
        payload: &[u8],
    ) {
        if !Self::should_dispatch_message(interceptors, topic, payload).await {
            return;
        }

        match topic_router.route(topic) {
            Some(listener) => listener.on_message(topic, payload).await,
            None => {
                #[cfg(feature = "trace-log")]
                warn!("No MQTT topic listener found for topic {}", topic);
            }
        }
    }

    async fn should_dispatch_message(
        interceptors: &[Box<dyn MessageInterceptor>],
        topic: &str,
        payload: &[u8],
    ) -> bool {
        for interceptor in interceptors {
            if !interceptor.message_entry(topic, payload).await {
                return false;
            }
        }

        true
    }

    async fn handle_successful_reconnect(reconnect_client: &AsyncClient, topics: &[Topic]) {
        if topics.is_empty() {
            warn!("Client reconnected successfully");
            return;
        }

        if let Err(error) = reconnect_client
            .subscribe_many(Self::build_subscribe_filters(topics))
            .await
        {
            error!(
                "Failed to resubscribe configured topics after reconnect: {:?}",
                error
            );
            return;
        }

        warn!("Client reconnected successfully, resubscribed configured topics");
    }

    fn subscribe_topics_try(client: &AsyncClient, topics: &[Topic]) -> Result<(), ClientError> {
        if topics.is_empty() {
            return Ok(());
        }

        client.try_subscribe_many(Self::build_subscribe_filters(topics))
    }

    fn build_subscribe_filters(topics: &[Topic]) -> Vec<SubscribeFilter> {
        topics
            .iter()
            .map(|topic| {
                let qos = topic.qos.map(resolve_qos).unwrap_or(QoS::AtLeastOnce);
                SubscribeFilter::new(topic.topic.clone(), qos)
            })
            .collect()
    }

    fn next_retries_remaining(retries_remaining: usize, error: &ConnectionError) -> usize {
        if retries_remaining == 0 || !is_connection_refused(error) {
            return retries_remaining;
        }

        retries_remaining - 1
    }

    fn request_graceful_shutdown() {
        if let Some(tx) = APPLICATION_GRACEFUL_SHUTDOWN_SIGNAL.get() {
            if let Err(error) = tx.send(()) {
                error!("Failed to send graceful shutdown signal: {:?}", error);
            }
        }
    }

    async fn publish_inner<S, V>(
        &self,
        topic: S,
        qos: QoS,
        retain: bool,
        message: V,
    ) -> Result<(), ClientError>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        self.client.publish(topic, qos, retain, message).await
    }

    /// Publishes a message with default QoS `AtLeastOnce`.
    pub async fn publish<S, V>(&self, topic: S, message: V) -> Result<(), ClientError>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        self.publish_inner(topic, QoS::AtLeastOnce, false, message)
            .await
    }

    /// Publishes a message with a custom QoS level.
    pub async fn publish_with_qos_value<S, V>(
        &self,
        topic: S,
        q: u8,
        message: V,
    ) -> Result<(), ClientError>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        self.publish_inner(topic, resolve_qos(q), false, message)
            .await
    }

    /// Publishes a message with a custom QoS level and retain flag.
    pub async fn publish_with_retain_value<S, V>(
        &self,
        topic: S,
        q: u8,
        retain: bool,
        message: V,
    ) -> Result<(), ClientError>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        self.publish_inner(topic, resolve_qos(q), retain, message)
            .await
    }

    /// Returns a reference to the MQTT client.
    pub fn get_client(&self) -> &AsyncClient {
        &self.client
    }

    /// Returns a reference to the MQTT properties.
    pub fn properties(&self) -> &MQTTClientProperties {
        &self.properties
    }
}

#[async_trait]
impl MQTTService for DefaultMQTTService {
    async fn publish<S, V>(&self, topic: S, message: V) -> Result<(), ClientError>
    where
        S: Into<String> + Send,
        V: Into<Vec<u8>> + Send,
    {
        self.publish_inner(topic, QoS::AtLeastOnce, false, message)
            .await
    }
    async fn publish_with_qos<S, V>(
        &self,
        topic: S,
        qos: QoS,
        message: V,
    ) -> Result<(), ClientError>
    where
        S: Into<String> + Send,
        V: Into<Vec<u8>> + Send,
    {
        self.publish_inner(topic, qos, false, message).await
    }

    async fn publish_with_retain<S, V>(
        &self,
        topic: S,
        qos: QoS,
        retain: bool,
        message: V,
    ) -> Result<(), ClientError>
    where
        S: Into<String> + Send,
        V: Into<Vec<u8>> + Send,
    {
        self.publish_inner(topic, qos, retain, message).await
    }
}

impl Deref for DefaultMQTTService {
    type Target = AsyncClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

fn resolve_qos(qos: u8) -> QoS {
    rumqttc::qos(qos).unwrap_or(QoS::AtLeastOnce)
}

impl_service!(DefaultMQTTService);
