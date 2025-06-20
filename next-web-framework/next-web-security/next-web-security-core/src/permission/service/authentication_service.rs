use crate::auth::models::login_type::LoginType;
use next_web_core::async_trait;

#[async_trait]
pub trait AuthenticationService<H = axum::http::HeaderMap, ID = String>: Send + Sync {
    fn id(&self, req_header: &H) -> ID;

    fn login_type(&self, req_header: &H) -> LoginType;

    /// Returns the roles of the user with the given `user_id` and `login_type`.
    async fn user_role(&self, user_id: &ID, login_type: &LoginType) -> Option<Vec<String>>;

    /// Returns the permission of the user with the given `user_id` and `login_type`.
    async fn user_permission(&self, user_id: &ID, login_type: &LoginType) ->  Option<Vec<String>>;
}
