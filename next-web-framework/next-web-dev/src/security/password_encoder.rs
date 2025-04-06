use bcrypt::{hash, verify, BcryptError, DEFAULT_COST};


#[derive(Clone)]
pub struct PasswordEncoder {
    salt: String,
}

impl PasswordEncoder {
    pub fn encode(&self, password: &str) -> Result<String, BcryptError> {
        let pwd = String::from(password) + self.salt.as_str();
        hash(pwd, DEFAULT_COST)
    }

    pub fn verify(&self, password: &str, hashed_password: &str) -> bool {
        let pwd = String::from(password) + self.salt.as_str();
        verify(pwd, hashed_password).unwrap_or(false)
    }
}

impl Default for PasswordEncoder {
    fn default() -> Self {
        Self {
            salt: String::from("next-web-dev-salt"),
        }
    }
}
