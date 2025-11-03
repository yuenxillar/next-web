
pub trait  AuthenticationDetailsSource: Send + Sync
{

    fn build_details(&self); 
}