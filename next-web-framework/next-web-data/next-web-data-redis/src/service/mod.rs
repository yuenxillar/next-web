use crate::properties::redis_properties::RedisClientProperties;
  #[cfg(feature = "lock")]
pub mod redis_lock_service;

pub mod redis_service;

pub(crate) fn gen_url(config: &RedisClientProperties, resp3: bool) -> String {
    let password = config.password();
    let host = config.host().unwrap_or("localhost".into());
    let port = config.port().unwrap_or(6379);

    // connect to redis
    let url = format!(
        "redis://{}{}:{}{}{}",
        password.map(|s| format!(":{}@", s)).unwrap_or_default(),
        host,
        port,
        format!("/{}", config.database().unwrap_or(0)),
        if resp3 { "?protocol=resp3" } else { "" }
    );
    url
}
