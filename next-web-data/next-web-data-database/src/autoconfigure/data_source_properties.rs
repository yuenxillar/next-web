use next_web_macros::properties;
use rudi_dev::singleton;

/// Base for configuration of a data source.
#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.datasource", dynamic)]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DataSourceProperties {
    /// Datasource name
    name: Option<String>,

    /// Database driver name.
    driver_name: String,

    /// URL of the database.
    url: String,

    /// Login username of the database.
    username: Option<String>,

    /// Login password of the database.
    password: Option<String>,

    /// Whether to generate a random datasource name.
    #[serde(default = "generate_unique_name")]
    generate_unique_name: bool,
}

fn generate_unique_name() -> bool {
    true
}

impl DataSourceProperties {
    /// Returns the `name` field.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the `driver_name` field.
    pub fn driver_name(&self) -> &str {
        self.driver_name.as_str()
    }

    /// Returns the `url` field.
    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    /// Returns the `username` field.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref().filter(|s| !s.is_empty())
    }

    /// Returns the `password` field.
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref().filter(|s| !s.is_empty())
    }

    /// Returns the `generate_unique_name` field.
    pub fn generate_unique_name(&self) -> bool {
        self.generate_unique_name
    }

    /// Sets the `name` field.
    pub fn set_name<V>(&mut self, value: V)
    where
        V: Into<Option<String>>,
    {
        self.name = value.into();
    }

    /// Sets the `driver_name` field.
    pub fn set_driver_name<V>(&mut self, value: V)
    where
        V: Into<String>,
    {
        self.driver_name = value.into();
    }

    /// Sets the `url` field.
    pub fn set_url<V>(&mut self, value: V)
    where
        V: Into<String>,
    {
        self.url = value.into();
    }

    /// Sets the `username` field.
    pub fn set_username<V>(&mut self, value: V)
    where
        V: Into<Option<String>>,
    {
        self.username = value.into();
    }

    /// Sets the `password` field.
    pub fn set_password<V>(&mut self, value: V)
    where
        V: Into<Option<String>>,
    {
        self.password = value.into();
    }

    /// Sets the `generate_unique_name` field.
    pub fn set_generate_unique_name(&mut self, value: bool) {
        self.generate_unique_name = value;
    }
}
