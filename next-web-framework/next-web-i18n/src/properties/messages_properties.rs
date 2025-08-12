use rudi_dev::{ Properties, Singleton};

#[Singleton(default, binds=[Self::into_properties])]
#[Properties( prefix = "next.messages")]
#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct MessagesProperties {
    local: Option<String>,
    base_name: Option<String>,
}

impl MessagesProperties {
    pub fn local(&self) -> Option<&str> {
        self.local.as_deref()
    }

    pub fn base_name(&self) -> Option<&str> {
        self.base_name.as_deref()
    }
}
