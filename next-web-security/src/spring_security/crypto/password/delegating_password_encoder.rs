use std::{collections::HashMap, sync::Arc};

use next_web_core::error::BoxError;

use crate::crypto::password::password_encoder::PasswordEncoder;

pub struct DelegatingPasswordEncoder {
    id_for_encode: String,
    id_to_password_encoder: HashMap<String, Arc<dyn PasswordEncoder>>,
    default_password_encoder_for_matches: Option<Arc<dyn PasswordEncoder>>,
}

impl DelegatingPasswordEncoder {
    pub fn new(
        id_for_encode: impl Into<String>,
        id_to_password_encoder: HashMap<String, Arc<dyn PasswordEncoder>>,
    ) -> Self {
        let id_for_encode = id_for_encode.into();
        assert!(
            id_to_password_encoder.contains_key(&id_for_encode),
            "id_for_encode must be found in id_to_password_encoder"
        );
        Self {
            id_for_encode,
            id_to_password_encoder,
            default_password_encoder_for_matches: None,
        }
    }

    pub fn set_default_password_encoder_for_matches(
        &mut self,
        password_encoder: Arc<dyn PasswordEncoder>,
    ) {
        self.default_password_encoder_for_matches = Some(password_encoder);
    }

    fn extract_id<'a>(&self, encoded_password: &'a str) -> Option<(&'a str, &'a str)> {
        if !encoded_password.starts_with('{') {
            return None;
        }
        let end = encoded_password.find('}')?;
        Some((&encoded_password[1..end], &encoded_password[end + 1..]))
    }
}

impl PasswordEncoder for DelegatingPasswordEncoder {
    fn encode(&self, raw_password: &str) -> Result<String, BoxError> {
        let encoder = self
            .id_to_password_encoder
            .get(&self.id_for_encode)
            .expect("id_for_encode must have a configured PasswordEncoder");
        Ok(format!(
            "{{{}}}{}",
            self.id_for_encode,
            encoder.encode(raw_password)?
        ))
    }

    fn matches(&self, raw_password: &str, encoded_password: &str) -> bool {
        if let Some((id, encoded)) = self.extract_id(encoded_password) {
            return self
                .id_to_password_encoder
                .get(id)
                .map(|encoder| encoder.matches(raw_password, encoded))
                .unwrap_or(false);
        }

        self.default_password_encoder_for_matches
            .as_ref()
            .map(|encoder| encoder.matches(raw_password, encoded_password))
            .unwrap_or(false)
    }

    fn upgrade_encoding(&self, encoded_password: &str) -> bool {
        let Some((id, encoded)) = self.extract_id(encoded_password) else {
            return true;
        };
        id != self.id_for_encode
            || self
                .id_to_password_encoder
                .get(id)
                .map(|encoder| encoder.upgrade_encoding(encoded))
                .unwrap_or(true)
    }
}

#[derive(Clone, Debug, Default)]
pub struct NoOpPasswordEncoder;

impl PasswordEncoder for NoOpPasswordEncoder {
    fn encode(&self, raw_password: &str) -> Result<String, BoxError> {
        Ok(raw_password.to_string())
    }

    fn matches(&self, raw_password: &str, encoded_password: &str) -> bool {
        raw_password == encoded_password
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use crate::crypto::password::{
        delegating_password_encoder::{DelegatingPasswordEncoder, NoOpPasswordEncoder},
        password_encoder::PasswordEncoder,
    };

    #[test]
    fn delegating_encoder_adds_id_prefix_and_matches_by_id() {
        let encoder = DelegatingPasswordEncoder::new(
            "noop",
            HashMap::from([(
                String::from("noop"),
                Arc::new(NoOpPasswordEncoder::default()) as Arc<dyn PasswordEncoder>,
            )]),
        );

        let encoded = encoder.encode("secret").unwrap();

        assert_eq!(encoded, "{noop}secret");
        assert!(encoder.matches("secret", &encoded));
        assert!(!encoder.matches("other", &encoded));
    }
}
