use next_web_core::async_trait;


#[async_trait]
pub trait GrantedAuthority
where
    Self: Send + Sync,
{

    async fn get_authority(&self) -> Option<String>;
}