use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{
    default_one_time_token::DefaultOneTimeToken,
    generate_one_time_token_request::GenerateOneTimeTokenRequest, one_time_token::OneTimeToken,
    one_time_token_authentication_token::OneTimeTokenAuthenticationToken,
    one_time_token_service::OneTimeTokenService,
};

type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;
type TokenGenerator = Arc<dyn Fn() -> String + Send + Sync>;

#[derive(Clone)]
pub struct InMemoryOneTimeTokenService {
    one_time_token_by_token: Arc<RwLock<HashMap<String, Arc<dyn OneTimeToken>>>>,
    clock: Clock,
    token_generator: TokenGenerator,
}

impl Default for InMemoryOneTimeTokenService {
    fn default() -> Self {
        Self {
            one_time_token_by_token: Arc::new(RwLock::new(HashMap::new())),
            clock: Arc::new(Utc::now),
            token_generator: Arc::new(|| Uuid::new_v4().to_string()),
        }
    }
}

impl InMemoryOneTimeTokenService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_clock<F>(&mut self, clock: F)
    where
        F: Fn() -> DateTime<Utc> + Send + Sync + 'static,
    {
        self.clock = Arc::new(clock);
    }

    pub fn set_token_generator<F>(&mut self, token_generator: F)
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        self.token_generator = Arc::new(token_generator);
    }

    pub fn len(&self) -> usize {
        match self.one_time_token_by_token.read() {
            Ok(tokens) => tokens.len(),
            Err(poisoned) => poisoned.into_inner().len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn clean_expired_tokens_if_needed(&self) {
        if self.len() < 100 {
            return;
        }
        let mut tokens = match self.one_time_token_by_token.write() {
            Ok(tokens) => tokens,
            Err(poisoned) => poisoned.into_inner(),
        };
        let now = (self.clock)();
        tokens.retain(|_, token| !is_expired_at(token.as_ref(), now));
    }
}

impl OneTimeTokenService for InMemoryOneTimeTokenService {
    fn generate(&self, request: GenerateOneTimeTokenRequest) -> Arc<dyn OneTimeToken> {
        let token = (self.token_generator)();
        let expires_at = (self.clock)() + request.expires_in();
        let ott: Arc<dyn OneTimeToken> = Arc::new(DefaultOneTimeToken::new(
            token.clone(),
            request.username(),
            expires_at,
        ));
        let mut tokens = match self.one_time_token_by_token.write() {
            Ok(tokens) => tokens,
            Err(poisoned) => poisoned.into_inner(),
        };
        tokens.insert(token, ott.clone());
        drop(tokens);
        self.clean_expired_tokens_if_needed();
        ott
    }

    fn consume(
        &self,
        authentication_token: &OneTimeTokenAuthenticationToken,
    ) -> Option<Arc<dyn OneTimeToken>> {
        let mut tokens = match self.one_time_token_by_token.write() {
            Ok(tokens) => tokens,
            Err(poisoned) => poisoned.into_inner(),
        };
        let token = tokens.remove(authentication_token.token_value())?;
        drop(tokens);
        if is_expired_at(token.as_ref(), (self.clock)()) {
            return None;
        }
        Some(token)
    }
}

fn is_expired_at(token: &dyn OneTimeToken, now: DateTime<Utc>) -> bool {
    now > token.expires_at()
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use crate::authentication::ott::{
        generate_one_time_token_request::GenerateOneTimeTokenRequest,
        in_memory_one_time_token_service::InMemoryOneTimeTokenService,
        one_time_token_authentication_token::OneTimeTokenAuthenticationToken,
        one_time_token_service::OneTimeTokenService,
    };

    #[test]
    fn generate_then_token_value_is_uuid_and_username_is_used() {
        let service = InMemoryOneTimeTokenService::new();

        let one_time_token = service.generate(GenerateOneTimeTokenRequest::new("user"));

        assert!(uuid::Uuid::parse_str(one_time_token.token_value()).is_ok());
        assert_eq!(one_time_token.username(), "user");
    }

    #[test]
    fn consume_when_token_does_not_exist_then_none() {
        let service = InMemoryOneTimeTokenService::new();
        let authentication_token = OneTimeTokenAuthenticationToken::new("123");

        assert!(service.consume(&authentication_token).is_none());
    }

    #[test]
    fn consume_when_token_exists_then_returns_it_once() {
        let service = InMemoryOneTimeTokenService::new();
        let generated = service.generate(GenerateOneTimeTokenRequest::new("user"));
        let authentication_token = OneTimeTokenAuthenticationToken::new(generated.token_value());

        let consumed = service.consume(&authentication_token).unwrap();

        assert_eq!(consumed.token_value(), generated.token_value());
        assert_eq!(consumed.username(), generated.username());
        assert!(service.consume(&authentication_token).is_none());
    }

    #[test]
    fn consume_when_token_is_expired_then_none() {
        let mut service = InMemoryOneTimeTokenService::new();
        let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        service.set_clock(move || now);
        let generated = service.generate(GenerateOneTimeTokenRequest::new("user"));

        service.set_clock(move || now + Duration::minutes(10));
        let authentication_token = OneTimeTokenAuthenticationToken::new(generated.token_value());

        assert!(service.consume(&authentication_token).is_none());
    }

    #[test]
    fn generate_when_more_than_one_hundred_tokens_then_cleans_expired() {
        let mut service = InMemoryOneTimeTokenService::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let token_counter = counter.clone();
        service.set_token_generator(move || {
            format!("token-{}", token_counter.fetch_add(1, Ordering::SeqCst))
        });

        let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        service.set_clock(move || now);
        for index in 0..50 {
            service.generate(GenerateOneTimeTokenRequest::new(format!("old-{index}")));
        }

        service.set_clock(move || now + Duration::minutes(2));
        for index in 0..50 {
            service.generate(GenerateOneTimeTokenRequest::new(format!("new-{index}")));
        }

        service.set_clock(move || now + Duration::minutes(6));
        service.generate(GenerateOneTimeTokenRequest::new("trigger-cleanup"));

        assert_eq!(service.len(), 51);
    }
}
