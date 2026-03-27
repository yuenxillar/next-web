use std::sync::Arc;

use hashbrown::HashMap;
use next_web_core::{
    async_trait, context::properties::ApplicationProperties, error::BoxError,
    traits::singleton::Singleton, ApplicationContext, AutoRegister,
};
use rudi_dev::singleton;

use crate::{
    core::{
        interceptor::message_interceptor::MessageInterceptor, route::TopicRoute,
        topic::base_topic::BaseTopic,
    },
    properties::mqtt_properties::MQTTClientProperties,
    service::mqtt_service::MQTTService,
};

#[singleton(binds = [Self::into_auto_register])]
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
    ) -> Result<(), BoxError> {
        let mqtt_properties = self.0.clone();

        if mqtt_properties == MQTTClientProperties::default() {
            return Ok(());
        }

        let base_topics = ctx.resolve_by_type::<Box<dyn BaseTopic>>();
        let mut route_map = HashMap::new();
        let mut route = Vec::new();

        for item in base_topics
            .into_iter()
            .filter(|item| !item.topic().is_empty())
        {
            let topic = item.topic();
            if topic.contains('#') || topic.contains('+') {
                route.push(
                    TopicRoute::new(topic, item)
                        .map_err(std::io::Error::other)?,
                );
            } else {
                route_map.insert(topic.into(), item);
            }
        }

        let var = ctx.resolve_option::<Box<dyn MessageInterceptor>>();
        let interceptor = if let Some(interceptor) = var {
            interceptor
        } else {
            ctx.resolve_with_name::<Box<dyn MessageInterceptor>>("defaultMQTTMessageInterceptor")
        };

        let mqtt_service = MQTTService::new(mqtt_properties, route_map, route, interceptor)?;
        let singleton_name = mqtt_service.singleton_name();
        ctx.insert_singleton_with_name(mqtt_service, singleton_name);

        Ok(())
    }
}
