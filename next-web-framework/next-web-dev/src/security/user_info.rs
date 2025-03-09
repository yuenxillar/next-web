use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    user_id: String,
    exp: usize,
}

impl UserInfo {
    pub fn user_id(&self) -> &String {
        &self.user_id
    }
    pub fn exp(&self) -> usize {
        self.exp
    }
}
