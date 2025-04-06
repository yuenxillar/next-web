use async_trait::async_trait;
use hashbrown::HashMap;
use rudi::Singleton;

use super::{
    authorization_service::AuthorizationService, login_type::LoginType, user_info::UserInfo,
};

#[derive(Clone)]
pub struct MemoryAuthService {
    users: HashMap<String, UserInfo>,
}

#[Singleton(name = "memoryAuthService")]
impl MemoryAuthService {
    #[di]
    pub fn new() -> Self {
        Self {
            users: HashMap::new()
        }
    }
}

#[async_trait]
impl AuthorizationService<Vec<String>> for MemoryAuthService {
    async fn user_role(&self, user_id: &String, login_type: &LoginType) -> Vec<String> {
        vec![]
    }

    async fn user_permission(&self, user_id: &String, login_type: &LoginType) -> Vec<String> {
        vec![]
    }

    async fn verify_token(&self, user_id: &String, login_type: &LoginType) -> bool {
        self.users.contains_key(user_id)
    }
}
