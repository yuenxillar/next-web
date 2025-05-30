use std::fmt::Debug;

use next_web_core::async_trait;


#[async_trait]
pub trait AuthenticationService<H = axum::http::HeaderMap>: Send + Sync {
    type Id: Default;

    type LoginType: Eq + PartialEq + Clone + Debug + Default;

    type AccessElement: AsRef<str> + ToString;

    fn id(&self, req_headers: &H) -> Self::Id;

    fn login_type(&self, req_headers: &H) -> Self::LoginType;

    /// Returns the roles of the user with the given `user_id` and `login_type`.
    async fn user_role(
        &self,
        user_id: &Self::Id,
        login_type: &Self::LoginType,
    ) -> Vec<Self::AccessElement>;

    /// Returns the permission of the user with the given `user_id` and `login_type`.
    async fn user_permission(
        &self,
        user_id: &Self::Id,
        login_type: &Self::LoginType,
    ) -> Vec<Self::AccessElement>;
}
