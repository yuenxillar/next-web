use crate::authentication::password::compromised_password_decision::CompromisedPasswordDecision;

pub trait CompromisedPasswordChecker: Send + Sync {
    fn check(&self, password: Option<&str>) -> CompromisedPasswordDecision;
}
