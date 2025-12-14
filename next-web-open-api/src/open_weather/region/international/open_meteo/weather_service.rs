
const APP_ID_ENV_VAR_NAME: &'static str = "OPEN_METEO_APP_ID";

#[derive(Clone)]
pub struct OpenMeteoWeatherService {
    pub(crate) api_key: Box<str>,
    pub(crate) client: reqwest::Client,
}

impl OpenMeteoWeatherService {
    pub fn new(api_key: impl Into<Box<str>>) -> Self {
        Self {
            api_key: api_key.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn from_client(api_key: impl Into<Box<str>>, client: reqwest::Client) -> Self {
        Self {
            api_key: api_key.into(),
            client,
        }
    }

    pub fn from_env() -> Self {
        let api_key = std::env::var(APP_ID_ENV_VAR_NAME).unwrap();
        Self {
            api_key: api_key.into(),
            client: reqwest::Client::new(),
        }
    }
}
