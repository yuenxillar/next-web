use hashbrown::HashMap;
use next_web_core::{
    async_trait, context::properties::ApplicationProperties, ApplicationContext, AutoRegister,
};
use rudi::SingleOwner;

use crate::{
    core::topic::base_topic::BaseTopic, properties::mqtt_properties::MQTTClientProperties,
    service::mqtt_service::MQTTService,
};

#[SingleOwner(binds = [Self::into_auto_register])]
pub struct MqttServiceAutoRegister(pub MQTTClientProperties);

impl MqttServiceAutoRegister {
    fn into_auto_register(self) -> Box<dyn AutoRegister> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoRegister for MqttServiceAutoRegister {
    fn singleton_name(&self) -> &'static str {
        "mqttService"
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mqtt_properties = self.0.clone();

        let base_topics = ctx.resolve_by_type::<Box<dyn BaseTopic>>();
        let mut map = HashMap::new();
        base_topics.into_iter().for_each(|item| {
            map.insert(item.topic().into(), item);
        });

        let mqtt_service = MQTTService::new(mqtt_properties, map);
        ctx.insert_singleton_with_name(mqtt_service, self.singleton_name());

        Ok(())
    }
}
