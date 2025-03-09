use async_trait::async_trait;

use super::login_type::LoginType;

#[async_trait]
pub trait AuthorizationService<V: IntoIterator<Item = String>>: Send + Sync + 'static {

    /// Returns the roles of the user with the given `user_id` and `login_type`.
    async fn user_role(&self, user_id: &String, login_type: &LoginType) -> V;

    /// Returns the permission of the user with the given `user_id` and `login_type`.
    async fn user_permission(&self, user_id: &String, login_type: &LoginType) -> V;

    /// Verifies the token of the user with the given `user_id` and `login_type`.
    async fn verify_token(&self, user_id: &String, login_type: &LoginType) -> bool;
}
