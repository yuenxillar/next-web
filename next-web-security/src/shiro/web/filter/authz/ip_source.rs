
pub trait IpSource
where 
Self: Send + Sync
{
    fn get_authorized_ips(&self) -> Vec<&str>;

    fn get_denied_ips(&self) -> Vec<&str>;
}
