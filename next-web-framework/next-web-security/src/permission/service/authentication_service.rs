use crate::auth::models::login_type::LoginType;
use next_web_core::async_trait;

use axum::http::HeaderMap;

#[async_trait]
pub trait AuthenticationService: Send + Sync {

    fn user_id(&self, req_header: &HeaderMap) -> String;

    fn login_type(&self, req_header: &HeaderMap) -> LoginType;

    /// Returns the roles of the user with the given `user_id` and `login_type`.
    async fn user_role(&self, user_id: &str, login_type: &LoginType) -> Option<Vec<String>>;

    /// Returns the permission of the user with the given `user_id` and `login_type`.
    async fn user_permission(&self, user_id: &str, login_type: &LoginType) ->  Option<Vec<String>>;
}
