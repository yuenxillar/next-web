use crate::{
    core::{
        interceptor::message_interceptor::MessageInterceptor,
        router::{MacthType, TopicRouter},
        topic::base_topic::BaseTopic,
    },
    properties::mqtt_properties::MQTTClientProperties,
};

use hashbrown::HashMap;
use next_web_core::core::service::Service;
use rumqttc::{
    AsyncClient, ConnectReturnCode, Event, MqttOptions, NetworkOptions, Packet, QoS,
    SubscribeFilter,
};
use tracing::error;

/// MQTT Service
/// MQTT 服务
/// 
/// This struct provides MQTT client functionality including:
/// 这个结构体提供MQTT客户端功能，包括:
/// - Connection management 连接管理
/// - Topic subscription 主题订阅
/// - Message publishing 消息发布
/// - Message routing 消息路由
/// - Interception handling 拦截处理
#[derive(Clone)]
pub struct MQTTService {
    /// MQTT client configuration properties
    /// MQTT客户端配置属性
    properties: MQTTClientProperties,
    
    /// Async MQTT client instance
    /// 异步MQTT客户端实例
    client: AsyncClient,
}

impl Service for MQTTService {
    /// Returns the service name
    /// 返回服务名称
    fn service_name(&self) -> String {
        "mqttService".into()
    }
}

impl MQTTService {
    /// Creates a new MQTTService instance
    /// 创建新的MQTTService实例
    /// 
    /// # Arguments
    /// 参数
    /// - properties: MQTT client configuration 客户端配置
    /// - router_map: Topic handlers map 主题处理器映射
    /// - router: Topic routers 主题路由器
    /// - interceptor: Message interceptor 消息拦截器
    pub fn new(
        properties: MQTTClientProperties,
        router_map: HashMap<String, Box<dyn BaseTopic>>,
        router: Vec<TopicRouter>,
        interceptor: Box<dyn MessageInterceptor>,
    ) -> Self {
        let client = Self::build_client(&properties, router_map, router, interceptor);
        Self { properties, client }
    }

    /// Builds and configures the MQTT client
    /// 构建并配置MQTT客户端
    /// 
    /// Handles:
    /// 处理:
    /// - Connection options 连接选项
    /// - Topic subscription 主题订阅
    /// - Message event loop 消息事件循环
    /// - Error handling 错误处理
    fn build_client(
        properties: &MQTTClientProperties,
        mut base_topics: HashMap<String, Box<dyn BaseTopic>>,
        mut router: Vec<TopicRouter>,
        interceptor: Box<dyn MessageInterceptor>,
    ) -> AsyncClient {
        let mut options = MqttOptions::new(
            properties.client_id().unwrap_or("next-web-mqtt"),
            properties.host().unwrap_or("127.0.0.1"),
            properties.port().unwrap_or(1883),
        );

        options
            .set_keep_alive(std::time::Duration::from_millis(
                properties.keep_alive().unwrap_or(60000),
            ))
            .set_clean_session(properties.clean_session().unwrap_or(true))
            .set_credentials(
                properties.username().unwrap_or_default(),
                properties.password().unwrap_or_default(),
            );

        let (client, mut eventloop) = AsyncClient::new(options, 999);

        let mut network_options = NetworkOptions::new();
        network_options.set_connection_timeout(properties.connect_timeout().unwrap_or(5));
        eventloop.set_network_options(network_options);


        let topics = properties.topics();
        let client_1 = client.clone();

        // subscribe topics
        let var = topics
            .iter()
            .map(|item| SubscribeFilter::new(item.clone(), QoS::AtLeastOnce))
            .collect::<Vec<_>>();
        client.try_subscribe_many(var).unwrap();

        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(packet))) => {
                        let message = packet.payload;
                        let topic = packet.topic;
                        interceptor.message_entry(&topic, &message).await;

                        if let Some(basic) = base_topics.get_mut(&topic) {
                            basic.consume(&topic, &message).await;
                        }

                        for item in router.iter_mut() {
                            match item.match_type {
                                MacthType::Anything => {
                                    item.base_topic.consume(&topic, &message).await;
                                }

                                MacthType::Multilayer(index) => {
                                    if topic[0..index].eq(&item.topic[0..index]) {
                                        item.base_topic.consume(&topic, &message).await;
                                    }
                                }

                                MacthType::Singlelayer(left_index, right_index) => {
                                    let len = topic.len();
                                    if topic[0..left_index].eq(&item.topic[0..left_index]) {
                                        if right_index != 0 {
                                            if !topic[(len - right_index)..].eq(&item.topic[right_index..])
                                            {
                                                continue;
                                            }
                                        }
                                        item.base_topic.consume(&topic, &message).await;
                                    }
                                }
                            }
                        }
                    }

                    Ok(Event::Incoming(Packet::ConnAck(ack))) => {
                        // This generally refers to the need to receive ack information and re subscribe to the topic after reconnection
                        match ack.code {
                            ConnectReturnCode::Success => {
                                for topic in topics.iter() {
                                    client_1.subscribe(topic, QoS::AtLeastOnce).await.unwrap();
                                }
                            }
                            _ => {}
                        }
                    }

                    Err(e) => {
                        error!("Mqtt eventloop error, connection error case: {:?}", e);
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }

                    _ => {
                        // dodo -> Outgoing
                    }
                }
            }
        });

        client
    }

    /// Publishes a message to a topic with default QoS (AtLeastOnce)
    /// 使用默认QoS(AtLeastOnce)向主题发布消息
    /// 
    /// # Arguments
    /// 参数
    /// - topic: Target topic 目标主题
    /// - message: Message content 消息内容
    pub async fn publish<M: Into<Vec<u8>>>(&self, topic: &str, message: M) {
        let _ = self
            .client
            .publish(topic, QoS::AtLeastOnce, false, message)
            .await;
    }

    /// Publishes a message with custom QoS level
    /// 使用自定义QoS级别发布消息
    /// 
    /// # Arguments
    /// 参数
    /// - topic: Target topic 目标主题
    /// - q: QoS level (0-2) QoS级别(0-2)
    /// - message: Message content 消息内容
    pub async fn publish_and_qos<M: Into<Vec<u8>>>(&self, topic: &str, q: u8, message: M) {
        if let Ok(qos) = rumqttc::qos(q) {
            let _ = self.client.publish(topic, qos, false, message).await;
        }
    }

    /// Returns a reference to the MQTT client
    /// 返回MQTT客户端的引用
    pub fn get_client(&self) -> &AsyncClient {
        &self.client
    }

    /// Returns a reference to the MQTT properties
    /// 返回MQTT配置属性的引用
    pub fn properties(&self) -> &MQTTClientProperties {
        &self.properties
    }
}
