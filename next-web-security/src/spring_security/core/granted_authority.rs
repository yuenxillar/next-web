use next_web_core::async_trait;

use crate::core::Authentication;

#[async_trait]
pub trait GrantedAuthority
where
    Self: Send + Sync,
{
    async fn get_authority(&self) -> Option<String>;

    /// If this `GrantedAuthority` is a `SwitchUserGrantedAuthority`, returns
    /// the source (original) `Authentication`. Otherwise returns `None`.
    fn as_switch_user_source(&self) -> Option<std::sync::Arc<dyn Authentication>> {
        None
    }
}
