use std::error::Error;

use amqprs::{
    BasicProperties,
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicAckArguments, BasicConsumeArguments, BasicNackArguments, BasicPublishArguments,
        Channel, QueueBindArguments, QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
};
use next_web_core::{
    error::BoxError,
    traits::{service::Service, singleton::Singleton},
};
use tracing::{error, info, warn};

use crate::{
    autoconfigure::rabbitmq_properties::RabbitmqProperties,
    config::default_rabbitmq_listener_registration::DefaultRabbitmqListenerRegistration,
    core::binding::RabbitmqEndpoint,
};

/// RabbitMQ service used by the framework for publishing and consuming messages.
#[derive(Clone)]
pub struct RabbitmqService {
    properties: RabbitmqProperties,
    connection: Connection,
    channel: Channel,
}

impl Singleton for RabbitmqService {}
impl Service for RabbitmqService {}

impl RabbitmqService {
    pub async fn new(
        properties: RabbitmqProperties,
        endpoints: Vec<RabbitmqEndpoint>,
    ) -> Result<Self, amqprs::error::Error> {
        let (connection, channel) = Self::build_channel(&properties, &endpoints).await?;
        Ok(Self {
            properties,
            connection,
            channel,
        })
    }

    pub async fn start_listeners(
        &self,
        registrations: &[DefaultRabbitmqListenerRegistration],
    ) -> Result<(), Box<dyn Error>> {
        for registration in registrations {
            self.start_listener(registration).await?;
        }
        Ok(())
    }

    async fn start_listener(
        &self,
        registration: &DefaultRabbitmqListenerRegistration,
    ) -> Result<(), Box<dyn Error>> {
        let channel = self.connection.open_channel(None).await?;
        channel.register_callback(DefaultChannelCallback).await?;

        let consumer_tag = registration.consumer_tag_value().to_owned();
        let consume_arguments =
            BasicConsumeArguments::new(registration.endpoint().queue_name(), consumer_tag.as_str())
                .manual_ack(self.properties.manual_ack())
                .finish();

        let (server_consumer_tag, mut receiver) =
            channel.basic_consume_rx(consume_arguments).await?;
        let listener = registration.listener();
        let queue_name = registration.endpoint().queue_name().to_owned();
        let manual_ack = self.properties.manual_ack();
        let requeue_rejected = self.properties.requeue_rejected();

        tokio::spawn(async move {
            info!(
                "RabbitMQ consumer `{}` started for queue `{}`",
                server_consumer_tag, queue_name
            );

            while let Some(message) = receiver.recv().await {
                let delivery_tag = message
                    .deliver
                    .as_ref()
                    .map(|deliver| deliver.delivery_tag());

                match listener.on_message(message).await {
                    Ok(()) => {
                        if manual_ack {
                            if let Some(delivery_tag) = delivery_tag {
                                if let Err(err) = channel
                                    .basic_ack(BasicAckArguments::new(delivery_tag, false))
                                    .await
                                {
                                    error!(
                                        "Failed to ack RabbitMQ message on queue `{}`: {}",
                                        queue_name, err
                                    );
                                }
                            } else {
                                warn!(
                                    "RabbitMQ manual ack is enabled but delivery tag is missing for queue `{}`",
                                    queue_name
                                );
                            }
                        }
                    }
                    Err(err) => {
                        error!(
                            "RabbitMQ listener failed on queue `{}`: {}",
                            queue_name, err
                        );

                        if manual_ack {
                            if let Some(delivery_tag) = delivery_tag {
                                if let Err(nack_err) = channel
                                    .basic_nack(BasicNackArguments::new(
                                        delivery_tag,
                                        false,
                                        requeue_rejected,
                                    ))
                                    .await
                                {
                                    error!(
                                        "Failed to nack RabbitMQ message on queue `{}`: {}",
                                        queue_name, nack_err
                                    );
                                }
                            } else {
                                warn!(
                                    "RabbitMQ manual ack is enabled but delivery tag is missing for queue `{}`",
                                    queue_name
                                );
                            }
                        }
                    }
                }
            }

            info!(
                "RabbitMQ consumer `{}` stopped for queue `{}`",
                server_consumer_tag, queue_name
            );
        });

        Ok(())
    }

    async fn build_channel(
        properties: &RabbitmqProperties,
        endpoints: &[RabbitmqEndpoint],
    ) -> Result<(Connection, Channel), amqprs::error::Error> {
        let mut connection_arguments = OpenConnectionArguments::new(
            properties.host(),
            properties.port(),
            properties.username(),
            properties.password(),
        );
        connection_arguments.virtual_host(properties.virtual_host());

        let connection = Connection::open(&connection_arguments).await?;
        connection
            .register_callback(DefaultConnectionCallback)
            .await?;

        let channel = connection.open_channel(None).await?;
        channel.register_callback(DefaultChannelCallback).await?;

        for endpoint in endpoints {
            let exchange_arguments = amqprs::channel::ExchangeDeclareArguments::new(
                endpoint.exchange_name(),
                endpoint.exchange_type(),
            )
            .durable(endpoint.exchange_durable())
            .finish();
            channel.exchange_declare(exchange_arguments).await?;

            let queue_arguments = QueueDeclareArguments::new(endpoint.queue_name())
                .durable(endpoint.queue_durable())
                .finish();
            let declared_queue = channel.queue_declare(queue_arguments).await?;
            let queue_name = declared_queue
                .map(|(queue_name, _, _)| queue_name)
                .unwrap_or_else(|| endpoint.queue_name().to_owned());

            channel
                .queue_bind(QueueBindArguments::new(
                    &queue_name,
                    endpoint.exchange_name(),
                    endpoint.routing_key(),
                ))
                .await?;

            info!(
                "RabbitMQ endpoint ready: queue=`{}`, exchange=`{}`, routing_key=`{}`",
                queue_name,
                endpoint.exchange_name(),
                endpoint.routing_key()
            );
        }

        Ok((connection, channel))
    }

    pub async fn send_message<M: Into<Vec<u8>>>(
        &self,
        exchange: &str,
        routing_key: &str,
        message: M,
    ) -> Result<(), amqprs::error::Error> {
        let arguments = BasicPublishArguments::new(exchange, routing_key);
        self.channel
            .basic_publish(BasicProperties::default(), message.into(), arguments)
            .await
    }

    pub async fn send_message_with_properties<M: Into<Vec<u8>>>(
        &self,
        exchange: &str,
        routing_key: &str,
        message: M,
        properties: BasicProperties,
    ) -> Result<(), amqprs::error::Error> {
        let arguments = BasicPublishArguments::new(exchange, routing_key);
        self.channel
            .basic_publish(properties, message.into(), arguments)
            .await
    }

    pub async fn ack(&self, delivery_tag: u64) -> Result<(), amqprs::error::Error> {
        self.channel
            .basic_ack(BasicAckArguments::new(delivery_tag, false))
            .await
    }

    pub async fn nack(&self, delivery_tag: u64, requeue: bool) -> Result<(), amqprs::error::Error> {
        self.channel
            .basic_nack(BasicNackArguments::new(delivery_tag, false, requeue))
            .await
    }

    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn properties(&self) -> &RabbitmqProperties {
        &self.properties
    }
}
