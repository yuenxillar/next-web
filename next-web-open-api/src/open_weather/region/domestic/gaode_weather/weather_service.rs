const APP_ID_ENV_VAR_NAME: &'static str = "GAODE_WEATHER_APP_ID";

#[derive(Clone)]
pub struct GaodeWeatherService {
    pub(crate) key: Box<str>,
    pub(crate) client: reqwest::Client,
}

impl GaodeWeatherService {
    pub fn new(key: impl Into<Box<str>>) -> Self {
        Self {
            key: key.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn from_client(key: impl Into<Box<str>>, client: reqwest::Client) -> Self {
        Self {
            key: key.into(),
            client,
        }
    }

    pub fn from_env() -> Self {
        let key = std::env::var(APP_ID_ENV_VAR_NAME).unwrap();
        Self {
            key: key.into(),
            client: reqwest::Client::new(),
        }
    }
}
