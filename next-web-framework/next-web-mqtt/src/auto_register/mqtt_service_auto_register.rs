use std::sync::Arc;

use hashbrown::HashMap;
use next_web_core::{
    async_trait,
    context::properties::ApplicationProperties,
    core::service::{self, Service},
    ApplicationContext, AutoRegister,
};
use rudi_dev::Singleton;
use serde::ser;

use crate::{
    core::{
        interceptor::message_interceptor::MessageInterceptor, router::TopicRouter,
        topic::base_topic::BaseTopic,
    },
    properties::mqtt_properties::MQTTClientProperties,
    service::mqtt_service::MQTTService,
};

/// 定义一个单例拥有者，并绑定到 `into_auto_register` 方法
/// Define a singleton owner and bind it to the `into_auto_register` method
#[Singleton(binds = [Self::into_auto_register])]
#[derive(Clone)]
pub struct MqttServiceAutoRegister(pub MQTTClientProperties);

impl MqttServiceAutoRegister {
    /// 将当前结构体转换为 `AutoRegister` 的动态分发类型
    /// Convert the current structure into a dynamically dispatched `AutoRegister` type
    fn into_auto_register(self) -> Arc<dyn AutoRegister> {
        Arc::new(self)
    }
}

#[async_trait]
impl AutoRegister for MqttServiceAutoRegister {
    /// 返回单例名称，用于标识服务
    /// Return the singleton name used to identify the service
    fn singleton_name(&self) -> &'static str {
        ""
    }

    /// 异步注册方法，用于在应用上下文中注册 MQTT 服务
    /// Asynchronous registration method used to register the MQTT service in the application context
    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 克隆 MQTT 配置属性
        // Clone the MQTT configuration properties
        let mqtt_properties = self.0.clone();

        // 从上下文中解析所有实现了 `BaseTopic` 的组件
        // Resolve all components implementing `BaseTopic` from the context

        let base_topics = ctx.resolve_by_type::<Box<dyn BaseTopic>>();
        let mut router_map = HashMap::new();
        let mut router = Vec::new();
        base_topics.into_iter().for_each(|item| {
            let topic = item.topic();
            if topic.contains("#") || topic.contains("+") {
                router.push(TopicRouter::new(topic, item));
            } else {
                // 遍历所有 `BaseTopic` 并将其主题名插入哈希表
                // Iterate over all `BaseTopic` and insert their topic names into the hash map
                router_map.insert(topic.into(), item);
            }
        });

        // 尝试从上下文中解析消息拦截器
        // Attempt to resolve a message interceptor from the context
        let var = ctx.resolve_option::<Box<dyn MessageInterceptor>>();

        let interceptor = if let Some(var1) = var {
            // 如果解析成功，直接使用解析到的拦截器
            // If resolution succeeds, use the resolved interceptor directly
            var1
        } else {
            // 否则，尝试通过名称解析默认的消息拦截器
            // Otherwise, attempt to resolve the default message interceptor by name
            ctx.resolve_with_name::<Box<dyn MessageInterceptor>>("defaultMQTTMessageInterceptor")
        };

        // 创建 MQTT 服务实例，传入配置、主题映射和消息拦截器
        // Create an MQTT service instance, passing in the configuration, topic mapping, and message interceptor
        let mqtt_service = MQTTService::new(mqtt_properties, router_map, router, interceptor);

        // 将 MQTT 服务插入上下文，并命名为单例名称
        // Insert the MQTT service into the context and name it with the singleton name
        let service_name = mqtt_service.service_name();
        ctx.insert_singleton_with_name(mqtt_service, service_name);

        Ok(())
    }
}
