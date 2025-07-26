use std::ops::Deref;

use crate::{
    core::{
        interceptor::message_interceptor::MessageInterceptor,
        route::{MacthType, TopicRoute},
        topic::base_topic::BaseTopic,
    },
    properties::mqtt_properties::MQTTClientProperties,
};

use hashbrown::HashMap;
use next_web_core::interface::{service::Service, singleton::Singleton};
use rumqttc::{
    AsyncClient, ConnectReturnCode, Event, MqttOptions, NetworkOptions, Packet, QoS,
    SubscribeFilter,
};
use tracing::{error, warn};

/// MQTT Service
/// This struct provides MQTT client functionality including:
/// - Connection management
/// - Topic subscription
/// - Message publishing
/// - Message routing
/// - Interception handling
///
/// MQTT 服务
/// 这个结构体提供MQTT客户端功能，包括:
/// - 连接管理
/// - 主题订阅
/// - 消息发布
/// - 消息路由
/// - 拦截处理
#[derive(Clone)]
pub struct MQTTService {
    /// MQTT client configuration properties
    ///
    /// MQTT客户端配置属性
    properties: MQTTClientProperties,

    /// Async MQTT client instance
    ///
    /// 异步MQTT客户端实例
    client: AsyncClient,
}

impl Singleton  for MQTTService {}
impl Service    for MQTTService {}

impl MQTTService {
    /// Creates a new MQTTService instance
    ///
    /// # Arguments
    /// - `properties`: MQTT client configuration
    /// - `route_map`: Topic consumer map
    /// - `route`: Topic route
    /// - `interceptor`: Message interceptor
    ///
    /// # Returns
    /// Self
    ///
    /// 创建新的MQTTService实例
    ///
    /// # 参数
    /// - `properties`: MQTT客户端配置
    /// - `router_map`: 主题路由映射
    /// - `router`: 主题路由
    /// - `interceptor`: 消息拦截器
    ///
    /// # 返回值
    /// Self
    pub fn new(
        properties: MQTTClientProperties,
        route_map: HashMap<String, Box<dyn BaseTopic>>,
        route: Vec<TopicRoute>,
        interceptor: Box<dyn MessageInterceptor>,
    ) -> Self {
        let client = Self::build_client(&properties, route_map, route, interceptor);
        Self { properties, client }
    }

    /// Builds and configures the MQTT client
    ///
    /// # Arguments
    /// - `Connection options`
    /// - `Topic subscription`
    /// - `Message event loop`
    ///
    /// # Returns
    /// Async MQTT client instance
    ///
    /// 构建并配置MQTT客户端
    ///
    /// # 参数
    /// - `Connection options` 连接选项
    /// - `Topic subscription` 主题订阅
    /// - `Message event loop` 消息事件循环
    ///
    /// # 返回值
    /// 异步MQTT客户端实例
    ///
    fn build_client(
        properties: &MQTTClientProperties,
        mut route_map: HashMap<String, Box<dyn BaseTopic>>,
        mut route: Vec<TopicRoute>,
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
        let need_subscribe_topics = topics
            .iter()
            .map(|topic| {
                if topic.len() > 2 {
                    let two_char = &topic[topic.len() - 2..];
                    let qos: QoS = match two_char {
                        ":0" => QoS::AtMostOnce,
                        ":1" => QoS::AtLeastOnce,
                        ":2" => QoS::ExactlyOnce,
                        _ => QoS::AtLeastOnce,
                    };
                    return SubscribeFilter::new(topic.clone(), qos);
                }
                SubscribeFilter::new(topic.clone(), QoS::AtLeastOnce)
            })
            .collect::<Vec<_>>();
        client.try_subscribe_many(need_subscribe_topics).unwrap();

        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(packet))) => {
                        let message = packet.payload;
                        let topic = packet.topic;
                        if !interceptor.message_entry(&topic, &message).await {
                            continue;
                        }

                        if let Some(basic) = route_map.get_mut(&topic) {
                            basic.consume(&topic, &message).await;
                        }

                        for item in route.iter_mut() {
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
                                            if !topic[(len - right_index)..]
                                                .eq(&item.topic[right_index..])
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
                                warn!("Client reconnection successful, try re subscribing to the themes")
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
    ///
    /// # Arguments
    /// - `topic`: Target topic
    /// - `message`: Message content
    ///
    /// # Returns
    /// Asynchronous response result obtained from publishing operation
    ///
    ///
    /// 使用默认QoS(AtLeastOnce)向主题发布消息
    ///
    /// # 参数
    /// - `topic`: 目标主题
    /// - `message`: 消息内容
    ///
    /// # 返回值
    /// 发布操作得到的异步响应结果
    pub async fn publish<S, V>(&self, topic: S, message: V) -> Result<(), rumqttc::ClientError>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        self.client
            .publish(topic, QoS::AtLeastOnce, false, message)
            .await
    }

    /// Publishes a message with custom QoS level
    ///
    /// # Arguments
    ///
    /// - `topic`: Target topic
    /// - `q`: QoS level (0 1 2)
    /// - `message`: Message content
    ///
    /// # Returns
    /// Asynchronous response result obtained from publishing operation
    ///
    /// 使用自定义QoS级别发布消息
    ///
    /// # 参数
    /// - `topic`:  目标主题
    /// - `q`:  QoS级别(0 1 2)
    /// - `message`: 消息内容
    ///
    /// # 返回值
    /// 发布操作得到的异步响应结果
    pub async fn publish_with_qos<S, V>(
        &self,
        topic: S,
        q: u8,
        message: V,
    ) -> Result<(), rumqttc::ClientError>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        let qos = rumqttc::qos(q).unwrap_or(QoS::AtLeastOnce);
        self.client.publish(topic, qos, false, message).await
    }

    /// Publish messages with custom QoS levels and whether to Retain
    ///
    /// # Arguments
    ///
    /// - `topic`: Target topic
    /// - `q`: QoS level (0 1 2)
    /// - `retain`: Whether to retain
    /// - `message`: Message content
    ///
    /// # Returns
    /// Asynchronous response result obtained from publishing operation
    ///
    /// 发布具有自定义QoS级别的消息以及是否保留
    ///
    /// # 参数
    /// - `topic`:  目标主题
    /// - `q`:  QoS级别(0 1 2)
    /// - `retain`: 是否保留
    /// - `message`: 消息内容
    ///
    /// # 返回值
    /// 发布操作得到的异步响应结果
    pub async fn publish_with_retain<S, V>(
        &self,
        topic: S,
        q: u8,
        retain: bool,
        message: V,
    ) -> Result<(), rumqttc::ClientError>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        let qos = rumqttc::qos(q).unwrap_or(QoS::AtLeastOnce);
        self.client.publish(topic, qos, retain, message).await
    }

    /// Returns a reference to the MQTT client
    ///
    /// 返回MQTT客户端的引用
    pub fn get_client(&self) -> &AsyncClient {
        &self.client
    }

    /// Returns a reference to the MQTT properties
    ///
    /// 返回MQTT配置属性的引用
    pub fn properties(&self) -> &MQTTClientProperties {
        &self.properties
    }
}

impl Deref for MQTTService {
    type Target = AsyncClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}
