use std::fmt;

use super::token::Token;

/// The default implementation of Token.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DefaultToken {
    key: String,
    key_creation_time: i64,
    extended_information: String,
}

impl DefaultToken {
    pub fn new(
        key: impl Into<String>,
        key_creation_time: i64,
        extended_information: impl Into<String>,
    ) -> Self {
        let key = key.into();
        assert!(!key.trim().is_empty(), "Key required");

        Self {
            key,
            key_creation_time,
            extended_information: extended_information.into(),
        }
    }
}

impl Token for DefaultToken {
    fn key(&self) -> &str {
        &self.key
    }

    fn key_creation_time(&self) -> i64 {
        self.key_creation_time
    }

    fn extended_information(&self) -> &str {
        &self.extended_information
    }
}

impl fmt::Display for DefaultToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DefaultToken[key={}; creation={}; extended={}]",
            self.key, self.key_creation_time, self.extended_information
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{DefaultToken, Token};

    #[test]
    fn default_token_exposes_constructor_values() {
        let token = DefaultToken::new("key", 42, "extended");

        assert_eq!(token.key(), "key");
        assert_eq!(token.key_creation_time(), 42);
        assert_eq!(token.extended_information(), "extended");
        assert_eq!(
            token.to_string(),
            "DefaultToken[key=key; creation=42; extended=extended]"
        );
    }
}
