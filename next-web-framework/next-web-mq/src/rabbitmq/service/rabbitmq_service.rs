use amqprs::channel::Channel;
use amqprs::BasicProperties;
use amqprs::consumer::DefaultConsumer;
use amqprs::channel::BasicConsumeArguments;
use next_web_core::core::service::Service;
use amqprs::channel::QueueBindArguments;
use amqprs::channel::QueueDeclareArguments;
use amqprs::callbacks::DefaultChannelCallback;
use amqprs::callbacks::DefaultConnectionCallback;
use amqprs::connection::Connection;
use amqprs::channel::BasicPublishArguments;
use amqprs::connection::OpenConnectionArguments;
use crate::rabbitmq::bind_exchange::BindExchange;

use tracing::{ error, info};

use crate::rabbitmq::properties::rabbitmq_properties::RabbitMQClientProperties;

///  Service
#[derive(Clone)]
pub struct RabbitmqService {
    properties: RabbitMQClientProperties,
    channel: Channel,
}

impl Service for RabbitmqService {}

impl RabbitmqService {
    pub async fn new(properties: RabbitMQClientProperties, binds: Vec<BindExchange>) -> Self {
        let channel = Self::build_channel(&properties, &binds).await;
        Self {
            properties,
            channel,
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

                        // Start consumer with given name
                        let args = BasicConsumeArguments::new(&queue_name, "example_basic_pub_sub");
                        match channel
                            .basic_consume(DefaultConsumer::new(args.no_ack), args)
                            .await
                        {
                            Ok(consumer_tag) => {
                                info!(
                                    "Consumer started for queue {}. Consumer tag: {}",
                                    queue_name, consumer_tag
                                );
                                // Process messages or manage the consumer as needed
                            }
                            Err(err) => {
                                error!(
                                    "Failed to start consumer for queue {}: {}",
                                    queue_name, err
                                );
                            }
                        }

                        // Create arguments for basic_publish
                        let args = BasicPublishArguments::new(
                            bind_exchange.exchange_name(),
                            bind_exchange.routing_key(),
                        );

                        // Publish the message
                        match channel
                            .basic_publish(BasicProperties::default(), "content".into(), args)
                            .await
                        {
                            Ok(_) => {
                                info!(
                                    "Message published to exchange {} with routing key {}.",
                                    bind_exchange.exchange_name(),
                                    bind_exchange.routing_key()
                                );
                            }
                            Err(err) => {
                                error!("Failed to publish message to exchange {} with routing key {}: {}", bind_exchange.exchange_name(), bind_exchange.routing_key(), err);
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

    pub fn get_channel(&self) -> &Channel {
        &self.channel
    }

    pub fn properties(&self) -> &RabbitMQClientProperties {
        &self.properties
    }
}
