use rudi_dev::{ Properties, Singleton};

#[Singleton(default, binds=[Self::into_properties])]
#[Properties( prefix = "next.messages")]
#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct MessagesProperties {
    local: Option<String>,
    encoding: Option<String>,
}

impl MessagesProperties {
    pub fn local(&self) -> Option<&str> {
        self.local.as_deref()
    }
    pub fn encoding(&self) -> Option<&str> {
        self.encoding.as_deref()
    }
}
