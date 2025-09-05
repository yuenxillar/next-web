
use crate::rabbitmq::core::bind_exchange::BindExchange;
use crate::rabbitmq::core::listener::rabbit_listener::RabbitListener;
use amqprs::callbacks::DefaultChannelCallback;
use amqprs::callbacks::DefaultConnectionCallback;
use amqprs::channel::BasicConsumeArguments;
use amqprs::channel::BasicPublishArguments;
use amqprs::channel::Channel;
use amqprs::channel::ConsumerMessage;
use amqprs::channel::QueueBindArguments;
use amqprs::channel::QueueDeclareArguments;
use amqprs::connection::Connection;
use amqprs::connection::OpenConnectionArguments;
use amqprs::BasicProperties;
use next_web_core::traits::service::Service;

use next_web_core::traits::singleton::Singleton;
use tracing::{error, info};

use crate::rabbitmq::properties::rabbitmq_properties::RabbitMQClientProperties;

///  Service
#[derive(Clone)]
pub struct RabbitmqService {
    properties: RabbitMQClientProperties,
    channel: Channel,
}


impl Singleton  for RabbitmqService {}
impl Service    for RabbitmqService {}

impl RabbitmqService {
    pub async fn new(properties: RabbitMQClientProperties, binds: Vec<BindExchange>) -> Self {
        let channel = Self::build_channel(&properties, &binds).await;
        Self {
            properties,
            channel,
        }
    }

    pub(crate) async fn spawn_consumer(&self, consumer: Vec<Box<dyn RabbitListener>>) {
        for mut item in consumer {
            let basic_consume_arguments =
                BasicConsumeArguments::new(&item.queue(), &item.consumer_tag());
            if let Ok((_ctag, mut rx)) = self.add_consumer(basic_consume_arguments).await {
                tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        item.on_message(msg).await;
                    }
                });
            }
        }
    }

    async fn build_channel(
        options: &RabbitMQClientProperties,
        binds: &Vec<BindExchange>,
    ) -> Channel {
        // open a connection to RabbitMQ server
        let mut properties = OpenConnectionArguments::new(
            options.host().unwrap_or("localhost"),
            options.port().unwrap_or(5672),
            options.username().unwrap_or("guest"),
            options.password().unwrap_or("guest"),
        );
        properties.virtual_host(options.virtual_host().unwrap_or("/"));

        let connection = Connection::open(&properties).await.unwrap();
        connection
            .register_callback(DefaultConnectionCallback)
            .await
            .unwrap();

        // open a channel on the connection
        let channel = connection.open_channel(None).await.unwrap();
        for bind_exchange in binds.iter() {
            channel
                .register_callback(DefaultChannelCallback)
                .await
                .unwrap();

            // Declare a queue
            match channel
                .queue_declare(QueueDeclareArguments::durable_client_named(
                    bind_exchange.queue_name(),
                ))
                .await
            {
                Ok(result) => {
                    if let Some((queue_name, _, _)) = result {
                        info!("Queue {} declared successfully!", queue_name);

                        // Bind the queue to exchange
                        match channel
                            .queue_bind(QueueBindArguments::new(
                                &queue_name,
                                bind_exchange.exchange_name(),
                                bind_exchange.routing_key(),
                            ))
                            .await
                        {
                            Ok(_) => {
                                info!("Queue {} bound to exchange {} with routing key {} successfully!", queue_name, bind_exchange.exchange_name(), bind_exchange.routing_key());
                            }
                            Err(err) => {
                                error!("Failed to bind queue {} to exchange {} with routing key {}: {}", queue_name, bind_exchange.exchange_name(), bind_exchange.routing_key(), err);
                            }
                        }
                    }
                }
                Err(err) => {
                    error!(
                        "Failed to declare queue {}: {}",
                        bind_exchange.queue_name(),
                        err
                    );
                }
            }
        }
        channel
    }

    /// basic_publish
    pub async fn send_message<M: Into<Vec<u8>>>(
        &self,
        exchange: &str,
        routing_key: &str,
        message: M,
    ) -> Result<(), amqprs::error::Error> {
        let args = BasicPublishArguments::new(exchange, routing_key);
        self.channel
            .basic_publish(BasicProperties::default(), message.into(), args)
            .await
    }

    /// basic_publish and properties
    pub async fn send_message_with_properties<M: Into<Vec<u8>>>(
        &self,
        exchange: &str,
        routing_key: &str,
        message: M,
        properties: BasicProperties,
    ) -> Result<(), amqprs::error::Error> {
        let args = BasicPublishArguments::new(exchange, routing_key);
        self.channel
            .basic_publish(properties, message.into(), args)
            .await
    }

    pub async fn add_consumer(
        &self,
        basic_consume_arguments: BasicConsumeArguments,
    ) -> Result<
        (
            String,
            tokio::sync::mpsc::UnboundedReceiver<ConsumerMessage>,
        ),
        amqprs::error::Error,
    > {
        self.channel.basic_consume_rx(basic_consume_arguments).await
    }

    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    pub fn properties(&self) -> &RabbitMQClientProperties {
        &self.properties
    }
}
