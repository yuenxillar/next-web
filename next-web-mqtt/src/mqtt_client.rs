use std::{error::Error, ops::Deref, sync::Arc};

use crate::{
    autoconfigure::mqtt_properties::{MQTTClientProperties, Topic},
    generate_client_id,
    interceptor::message_interceptor::MessageInterceptor,
    poll_error_handler::{
        MQTTPollErrorAction, MQTTPollErrorContext, MQTTPollErrorHandler, is_connection_refused,
    },
    topic_router::TopicRouter,
};

use next_web_core::impl_service;
use rumqttc::{
    AsyncClient, ClientError, ConnectReturnCode, ConnectionError, Event, EventLoop, MqttOptions,
    NetworkOptions, Packet, QoS, SubscribeFilter,
};
use tracing::{error, warn};

/// Broker URL a client connects to when the properties configure none.
pub const DEFAULT_BROKER_URL: &str = "tcp://127.0.0.1:1883";

/// Default keep alive interval in seconds.
pub const DEFAULT_KEEP_ALIVE: u64 = 60;

/// Default connection timeout in seconds.
pub const DEFAULT_CONNECT_TIMEOUT: u64 = 10;

/// Default capacity of the bounded channel between client and event loop.
pub const DEFAULT_CHANNEL_CAP: usize = 2048;

/// MQTT client.
///
/// The client is responsible for:
///
/// - configuring the MQTT client from the broker URL,
/// - subscribing the configured topics,
/// - dispatching published messages to every matching topic listener,
/// - invoking the message interceptors before dispatch.
#[derive(Clone)]
pub struct MqttClient {
    /// MQTT client configuration properties.
    properties: Arc<MQTTClientProperties>,

    /// Async MQTT client instance.
    client: AsyncClient,
}

impl MqttClient {
    /// Creates a new MQTT client.
    pub fn new(
        properties: MQTTClientProperties,
        topic_router: TopicRouter,
        interceptor: Vec<Box<dyn MessageInterceptor>>,
        poll_error_handler: Arc<dyn MQTTPollErrorHandler>,
    ) -> Result<Self, Box<dyn Error>> {
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
    ) -> Result<AsyncClient, Box<dyn Error>> {
        let options = Self::build_options(properties)?;
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

    /// Builds the client options out of the configured properties.
    ///
    /// The broker URL selects the transport and carries the address, so the two
    /// are resolved together.
    ///
    /// # Errors
    ///
    /// Returns an error when the configured broker URL is malformed or uses a
    /// scheme this build cannot serve.
    fn build_options(properties: &MQTTClientProperties) -> Result<MqttOptions, Box<dyn Error>> {
        let endpoint = BrokerEndpoint::parse(properties.url().unwrap_or(DEFAULT_BROKER_URL))?;

        let mut options = MqttOptions::new(
            properties.client_id().unwrap_or(generate_client_id(false)),
            endpoint.address,
            endpoint.port,
        );

        endpoint.transport.apply(&mut options)?;

        options
            .set_keep_alive(std::time::Duration::from_secs(
                properties.keep_alive().unwrap_or(DEFAULT_KEEP_ALIVE),
            ))
            .set_clean_session(properties.clean_session().unwrap_or(true))
            .set_credentials(
                properties.username().unwrap_or_default(),
                properties.password().unwrap_or_default(),
            );

        Ok(options)
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

        let listeners = topic_router.route(topic);

        if listeners.is_empty() {
            #[cfg(feature = "trace-log")]
            warn!("No MQTT topic listener found for topic {}", topic);
        } else {
            for listener in listeners {
                listener.on_message(topic, payload).await;
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

    /// Returns a reference to the MQTT properties.
    pub fn properties(&self) -> &MQTTClientProperties {
        &self.properties
    }
}

impl MqttClient {
    pub async fn publish<S, V>(&self, topic: S, message: V) -> Result<(), ClientError>
    where
        S: Into<String> + Send,
        V: Into<Vec<u8>> + Send,
    {
        self.publish_inner(topic, QoS::AtLeastOnce, false, message)
            .await
    }

    pub async fn publish_with_qos<S, V>(
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

    pub async fn publish_with_retain<S, V>(
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

impl Deref for MqttClient {
    type Target = AsyncClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

fn resolve_qos(qos: u8) -> QoS {
    rumqttc::qos(qos).unwrap_or(QoS::AtLeastOnce)
}

/// The broker endpoint parsed from the `host` URL.
struct BrokerEndpoint {
    /// The address handed to `MqttOptions::new`.
    ///
    /// A WebSocket endpoint keeps the full URL, because the handshake needs the
    /// host, the port and the path. Every other transport uses the bare host.
    address: String,
    /// The port parsed from the URL, or the default port of its scheme.
    port: u16,
    /// The transport the URL scheme selects.
    transport: BrokerTransport,
}

impl BrokerEndpoint {
    /// Parses a broker URL such as `tcp://127.0.0.1:1883` or
    /// `wss://broker.example.com:8000/mqtt`.
    ///
    /// # Errors
    ///
    /// Returns an error when the URL has no scheme, when the scheme is not
    /// supported, when the URL has no host, or when the port is not a number.
    fn parse(url: &str) -> Result<Self, Box<dyn Error>> {
        let (scheme, rest) = url
            .split_once("://")
            .ok_or_else(|| invalid_broker_url(url, "the URL has no scheme"))?;

        let transport = BrokerTransport::from_scheme(scheme)
            .ok_or_else(|| invalid_broker_url(url, "the scheme is not supported"))?;

        // Drop the path, the query and the fragment, which only a WebSocket
        // handshake uses, and the user information, which is configured
        // separately.
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        let authority = authority.rsplit('@').next().unwrap_or(authority);

        let (host, port) = split_authority(authority, transport.default_port())
            .map_err(|reason| invalid_broker_url(url, reason))?;

        if host.is_empty() {
            return Err(invalid_broker_url(url, "the URL has no host"));
        }

        let address = match transport {
            // A WebSocket handshake needs the full URL; the other transports
            // connect to the bare host.
            BrokerTransport::Ws | BrokerTransport::Wss => url.to_owned(),
            BrokerTransport::Tcp | BrokerTransport::Tls => host,
        };

        Ok(Self {
            address,
            port,
            transport,
        })
    }
}

/// The transport a broker URL scheme selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrokerTransport {
    /// Plain TCP (`tcp://`, `mqtt://`).
    Tcp,
    /// TCP over TLS (`ssl://`, `mqtts://`).
    Tls,
    /// Plain WebSocket (`ws://`).
    Ws,
    /// WebSocket over TLS (`wss://`).
    Wss,
}

impl BrokerTransport {
    /// Returns the transport a scheme selects, or `None` when it is unknown.
    fn from_scheme(scheme: &str) -> Option<Self> {
        match scheme.to_ascii_lowercase().as_str() {
            "tcp" | "mqtt" => Some(Self::Tcp),
            "ssl" | "mqtts" => Some(Self::Tls),
            "ws" => Some(Self::Ws),
            "wss" => Some(Self::Wss),
            _ => None,
        }
    }

    /// Returns the port used when the URL does not carry one.
    fn default_port(self) -> u16 {
        match self {
            Self::Tcp => 1883,
            Self::Tls => 8883,
            Self::Ws => 80,
            Self::Wss => 443,
        }
    }

    /// Applies the transport to the client options.
    ///
    /// # Errors
    ///
    /// Returns an error when a WebSocket scheme is used without the `ws`
    /// feature, which is the only scheme this build cannot serve.
    fn apply(self, options: &mut MqttOptions) -> Result<(), Box<dyn Error>> {
        match self {
            Self::Tcp => {}
            Self::Tls => {
                options.set_transport(rumqttc::Transport::tls_with_default_config());
            }
            Self::Ws => {
                #[cfg(feature = "ws")]
                options.set_transport(rumqttc::Transport::Ws);
                #[cfg(not(feature = "ws"))]
                return Err(ws_feature_required("ws"));
            }
            Self::Wss => {
                #[cfg(feature = "ws")]
                options.set_transport(rumqttc::Transport::wss_with_default_config());
                #[cfg(not(feature = "ws"))]
                return Err(ws_feature_required("wss"));
            }
        }

        Ok(())
    }
}

/// Splits the host and the port of a URL authority.
///
/// The port falls back to `default_port` when the authority does not carry one.
/// An IPv6 host is written in brackets, for example `[::1]:1883`.
fn split_authority(authority: &str, default_port: u16) -> Result<(String, u16), &'static str> {
    if let Some(bracketed) = authority.strip_prefix('[') {
        let (host, rest) = bracketed
            .split_once(']')
            .ok_or("the IPv6 host is not closed")?;
        let port = match rest.strip_prefix(':') {
            Some(port) => port.parse().map_err(|_| "the port is not a number")?,
            None => default_port,
        };

        return Ok((host.to_owned(), port));
    }

    match authority.rsplit_once(':') {
        Some((host, port)) => Ok((
            host.to_owned(),
            port.parse().map_err(|_| "the port is not a number")?,
        )),
        None => Ok((authority.to_owned(), default_port)),
    }
}

/// Builds the error reported for an unusable broker URL.
fn invalid_broker_url(url: &str, reason: &str) -> Box<dyn Error> {
    format!("invalid MQTT broker URL '{url}': {reason}").into()
}

/// Builds the error reported when a scheme needs a feature that is off.
#[cfg(not(feature = "ws"))]
fn ws_feature_required(scheme: &str) -> Box<dyn Error> {
    format!("the '{scheme}://' broker URL requires the `ws` feature of next-web-mqtt").into()
}

impl_service!(MqttClient);

#[cfg(test)]
mod tests {
    use super::{BrokerEndpoint, BrokerTransport};

    /// Parses a URL the tests expect to be valid.
    fn endpoint(url: &str) -> BrokerEndpoint {
        BrokerEndpoint::parse(url).expect("the URL is valid")
    }

    #[test]
    fn parses_a_tcp_url() {
        let endpoint = endpoint("tcp://127.0.0.1:9999");

        assert_eq!(endpoint.address, "127.0.0.1");
        assert_eq!(endpoint.port, 9999);
        assert_eq!(endpoint.transport, BrokerTransport::Tcp);
    }

    #[test]
    fn uses_the_default_port_of_the_scheme() {
        assert_eq!(endpoint("mqtt://broker.example.com").port, 1883);
        assert_eq!(endpoint("ssl://broker.example.com").port, 8883);
        assert_eq!(endpoint("ws://broker.example.com/mqtt").port, 80);
        assert_eq!(endpoint("wss://broker.example.com").port, 443);
    }

    #[test]
    fn keeps_the_websocket_url_and_drops_the_path_of_the_others() {
        assert_eq!(
            endpoint("ws://broker.example.com:8000/mqtt").address,
            "ws://broker.example.com:8000/mqtt"
        );
        assert_eq!(
            endpoint("wss://broker.example.com/mqtt").transport,
            BrokerTransport::Wss
        );
        assert_eq!(
            endpoint("tcp://broker.example.com:1883/path").address,
            "broker.example.com"
        );
    }

    #[test]
    fn parses_an_ipv6_host() {
        let endpoint = endpoint("tcp://[::1]:1883");

        assert_eq!(endpoint.address, "::1");
        assert_eq!(endpoint.port, 1883);
    }

    #[test]
    fn rejects_urls_without_a_supported_scheme() {
        assert!(BrokerEndpoint::parse("127.0.0.1:1883").is_err());
        assert!(BrokerEndpoint::parse("ftp://broker.example.com").is_err());
        assert!(BrokerEndpoint::parse("tcp://").is_err());
        assert!(BrokerEndpoint::parse("tcp://broker.example.com:not-a-port").is_err());
    }

    #[cfg(not(feature = "ws"))]
    #[test]
    fn rejects_a_websocket_url_without_the_feature() {
        let mut options = rumqttc::MqttOptions::new("client", "broker.example.com", 8000);

        assert!(BrokerTransport::Ws.apply(&mut options).is_err());
    }
}
