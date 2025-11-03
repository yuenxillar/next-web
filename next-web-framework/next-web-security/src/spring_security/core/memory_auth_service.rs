use next_web_core::async_trait;
use std::collections::HashMap;

use crate::auth::service::authentication_service::AuthenticationService;


#[derive(Clone)]
pub struct MemoryAuthService {
    users: HashMap<String, UserInfo>,
}

impl MemoryAuthService {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }
}

#[async_trait]
impl AuthenticationService for MemoryAuthService {
    type Id = String;

    type LoginType = ();

    type AccessElement = std::borrow::Cow<'static, str>;

    fn id(&self, req_headers: &axum::http::HeaderMap) -> Self::Id {
        req_headers
            .get("User-Id")
            .map(|var| var.to_str().unwrap_or_default())
            .map(|var1| var1.to_string())
            .unwrap_or(Self::Id::default())
    }

    fn login_type(&self, req_headers: &axum::http::HeaderMap) -> Self::LoginType {
        Default::default()
    }

    async fn user_role(
        &self,
        user_id: &Self::Id,
        login_type: &Self::LoginType,
    ) -> Vec<Self::AccessElement> {
        vec![]
    }

    async fn user_permission(
        &self,
        user_id: &Self::Id,
        login_type: &Self::LoginType,
    ) -> Vec<Self::AccessElement> {
        vec![]
    }
}
