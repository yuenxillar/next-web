use std::sync::Arc;

use hashbrown::HashMap;
use next_web_core::{
    async_trait, context::properties::ApplicationProperties, interface::singleton::Singleton,
    ApplicationContext, AutoRegister,
};
use rudi_dev::Singleton;

use crate::{
    core::{
        interceptor::message_interceptor::MessageInterceptor, route::TopicRoute,
        topic::base_topic::BaseTopic,
    },
    properties::mqtt_properties::MQTTClientProperties,
    service::mqtt_service::MQTTService,
};

#[Singleton(binds = [Self::into_auto_register])]
#[derive(Clone)]
pub struct MQTTServiceAutoRegister(pub MQTTClientProperties);

impl MQTTServiceAutoRegister {
    fn into_auto_register(self) -> Arc<dyn AutoRegister> {
        Arc::new(self)
    }
}

#[async_trait]
impl AutoRegister for MQTTServiceAutoRegister {
    fn registered_name(&self) -> &'static str {
        ""
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mqtt_properties = self.0.clone();

        if mqtt_properties == MQTTClientProperties::default() {
            return Ok(());
        }

        // 从上下文中解析所有实现了 `BaseTopic` 的组件
        // Resolve all components implementing `BaseTopic` from the context
        let base_topics = ctx.resolve_by_type::<Box<dyn BaseTopic>>();
        let mut route_map = HashMap::new();
        let mut route = Vec::new();

        // 根据主题选择不同的路由方式
        // Select different routing methods based on the topic
        base_topics.into_iter().for_each(|item| {
            let topic = item.topic();
            if topic.contains("#") || topic.contains("+") {
                route.push(TopicRoute::new(topic, item));
            } else {
                // 遍历所有 `BaseTopic` 并将其主题名插入哈希表，此处为静态主题例如 /test
                // Iterate over all `BaseTopic` and insert their topic names into the hash map, which is static topic such as /test
                route_map.insert(topic.into(), item);
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
        let mqtt_service = MQTTService::new(mqtt_properties, route_map, route, interceptor);

        // 将 MQTT 服务插入上下文，并命名为单例名称
        // Insert the MQTT service into the context and name it with the singleton name
        let singleton_name = mqtt_service.singleton_name();
        ctx.insert_singleton_with_name(mqtt_service, singleton_name);

        Ok(())
    }
}
