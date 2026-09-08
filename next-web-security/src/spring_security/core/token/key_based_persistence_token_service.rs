use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use rand::{rngs::OsRng, RngCore};

use super::{DefaultToken, Sha512DigestUtils, Token, TokenService};

type RandomBytes = Arc<dyn Fn(usize) -> Vec<u8> + Send + Sync>;
type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

#[derive(Clone)]
pub struct KeyBasedPersistenceTokenService {
    pseudo_random_number_bytes: usize,
    server_secret: String,
    server_integer: Option<i64>,
    random_bytes: RandomBytes,
    clock: Clock,
}

impl Default for KeyBasedPersistenceTokenService {
    fn default() -> Self {
        Self {
            pseudo_random_number_bytes: 32,
            server_secret: String::new(),
            server_integer: None,
            random_bytes: Arc::new(|size| {
                let mut bytes = vec![0_u8; size];
                OsRng.fill_bytes(&mut bytes);
                bytes
            }),
            clock: Arc::new(current_time_millis),
        }
    }
}

impl KeyBasedPersistenceTokenService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn after_properties_set(&self) {
        assert!(
            !self.server_secret.trim().is_empty(),
            "Server secret required"
        );
        assert!(self.server_integer.is_some(), "Server integer required");
    }

    pub fn set_server_secret(&mut self, server_secret: impl Into<String>) {
        self.server_secret = server_secret.into();
    }

    pub fn set_server_integer(&mut self, server_integer: i64) {
        assert!(server_integer != 0, "Server integer must not be zero");
        self.server_integer = Some(server_integer);
    }

    pub fn set_pseudo_random_number_bytes(&mut self, pseudo_random_number_bytes: usize) {
        self.pseudo_random_number_bytes = pseudo_random_number_bytes;
    }

    pub fn set_random_bytes_generator<F>(&mut self, random_bytes: F)
    where
        F: Fn(usize) -> Vec<u8> + Send + Sync + 'static,
    {
        self.random_bytes = Arc::new(random_bytes);
    }

    pub fn set_clock<F>(&mut self, clock: F)
    where
        F: Fn() -> i64 + Send + Sync + 'static,
    {
        self.clock = Arc::new(clock);
    }

    fn compute_key(&self, server_secret: &str, content: &str) -> String {
        let sha512_hex = Sha512DigestUtils::sha_hex(format!("{content}:{server_secret}"));
        let key_payload = format!("{content}:{sha512_hex}");
        STANDARD.encode(key_payload.as_bytes())
    }

    fn generate_pseudo_random_number(&self) -> String {
        hex::encode((self.random_bytes)(self.pseudo_random_number_bytes))
    }

    fn compute_server_secret_applicable_at(&self, time: i64) -> Option<String> {
        let server_integer = self.server_integer?;
        Some(format!("{}:{}", self.server_secret, time % server_integer))
    }

    fn parse_key_payload(&self, key: &str) -> Option<(i64, String, String, String)> {
        let decoded = STANDARD.decode(key.as_bytes()).ok()?;
        let payload = String::from_utf8(decoded).ok()?;
        let tokens = payload.split(':').collect::<Vec<_>>();
        if tokens.len() < 4 {
            return None;
        }

        let creation_time = tokens[0].parse::<i64>().ok()?;
        let pseudo_random_number = tokens[1].to_string();
        let sha512_hex = tokens.last()?.to_string();
        let extended_information = tokens[2..tokens.len() - 1].join(":");
        Some((
            creation_time,
            pseudo_random_number,
            extended_information,
            sha512_hex,
        ))
    }
}

impl TokenService for KeyBasedPersistenceTokenService {
    fn allocate_token(&self, extended_information: &str) -> Arc<dyn Token> {
        self.after_properties_set();

        let creation_time = (self.clock)();
        let server_secret = self
            .compute_server_secret_applicable_at(creation_time)
            .expect("server integer required");
        let pseudo_random_number = self.generate_pseudo_random_number();
        let content = format!("{creation_time}:{pseudo_random_number}:{extended_information}");
        let key = self.compute_key(&server_secret, &content);

        Arc::new(DefaultToken::new(key, creation_time, extended_information))
    }

    fn verify_token(&self, key: &str) -> Option<Arc<dyn Token>> {
        if key.is_empty() {
            return None;
        }
        self.after_properties_set();

        let (creation_time, pseudo_random_number, extended_information, presented_sha512_hex) =
            self.parse_key_payload(key)?;
        let server_secret = self.compute_server_secret_applicable_at(creation_time)?;
        let content = format!("{creation_time}:{pseudo_random_number}:{extended_information}");
        let expected_sha512_hex = Sha512DigestUtils::sha_hex(format!("{content}:{server_secret}"));

        if expected_sha512_hex != presented_sha512_hex {
            return None;
        }

        Some(Arc::new(DefaultToken::new(
            key.to_string(),
            creation_time,
            extended_information,
        )))
    }
}

fn current_time_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_millis() as i64
}
