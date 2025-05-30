use bcrypt::{hash, verify, BcryptError, DEFAULT_COST};

#[derive(Clone)]
pub struct PasswordEncoder;

impl PasswordEncoder {
    pub fn encode<'a>(
        &self,
        password: &'a str,
        salt: Option<&'a str>,
    ) -> Result<String, BcryptError> {
        let pwd = String::from(password) + salt.unwrap_or_default();
        hash(pwd, DEFAULT_COST)
    }

    pub fn verify<'a>(
        &self,
        password: &'a str,
        hashed_password: &'a str,
        salt: Option<&'a str>,
    ) -> bool {
        let pwd = String::from(password) + salt.unwrap_or_default();
        verify(pwd, hashed_password).unwrap_or(false)
    }
}
