const APP_ID_ENV_VAR_NAME: &'static str = "OPEN_WEATHER_APP_ID";

#[derive(Clone)]
pub struct OpenWeatherService {
    pub(crate) appid: Box<str>,
    pub(crate) client: reqwest::Client,
}

impl OpenWeatherService {
    pub fn new(appid: impl Into<Box<str>>) -> Self {
        Self {
            appid: appid.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn from_client(appid: impl Into<Box<str>>, client: reqwest::Client) -> Self {
        Self {
            appid: appid.into(),
            client,
        }
    }

    pub fn from_env() -> Self {
        let appid = std::env::var(APP_ID_ENV_VAR_NAME).unwrap();
        Self {
            appid: appid.into(),
            client: reqwest::Client::new(),
        }
    }
}
