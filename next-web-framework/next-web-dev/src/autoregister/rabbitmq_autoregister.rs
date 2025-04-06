use std::sync::Arc;

use futures::executor::block_on;
use next_web_core::autoregister::auto_register::AutoRegister;
use next_web_core::context::application_context::ApplicationContext;
use next_web_core::context::properties::ApplicationProperties;
use next_web_mq::amqprs::callbacks::DefaultChannelCallback;
use next_web_mq::amqprs::callbacks::DefaultConnectionCallback;
use next_web_mq::amqprs::channel::BasicConsumeArguments;
use next_web_mq::amqprs::channel::BasicPublishArguments;
use next_web_mq::amqprs::channel::Channel;
use next_web_mq::amqprs::channel::QueueBindArguments;
use next_web_mq::amqprs::channel::QueueDeclareArguments;
use next_web_mq::amqprs::connection::Connection;
use next_web_mq::amqprs::connection::OpenConnectionArguments;
use next_web_mq::amqprs::consumer::DefaultConsumer;
use next_web_mq::amqprs::BasicProperties;
use next_web_mq::rabbitmq::bind_exchange::BindExchange;
use next_web_mq::rabbitmq::bind_exchange::BindExchangeBuilder;
use next_web_mq::rabbitmq::core::client_properties::RabbitMQClientProperties;
use rbatis::async_trait;
use tracing::{error, info};

use crate::manager::rabbitmq_manager::RabbiqMQManager;

pub struct RabbitMQAutoregister(pub RabbitMQClientProperties);

#[async_trait]
impl AutoRegister for RabbitMQAutoregister {
    fn singleton_name(&self) -> &'static str {
        "RabbitMQAutoRegister"
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bind_exchange_builder = ctx.resolve_option::<Arc<dyn BindExchangeBuilder>>();
        if let Some(bind) = bind_exchange_builder {
            let bind_exchange = bind.value();

            let options = self.0.clone();
            let bind_exchanges = bind_exchange.clone();
            let channel = RabbitMQAutoregister::__register(options, bind_exchanges).await;

            let rabbitmq_manager = RabbiqMQManager::new(self.0.clone(), bind_exchange, channel);
            ctx.insert_singleton_with_name::<RabbiqMQManager, String>(
                rabbitmq_manager,
                String::from("rabbitmqManager"),
            );
        } else {
            error!("Failed to resolve BindExchangeBuilder");
        }

        Ok(())
    }
}

impl RabbitMQAutoregister {
    pub async fn __register(options: RabbitMQClientProperties, bind: Vec<BindExchange>) -> Channel {
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
        for bind_exchange in bind.iter() {
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
}
