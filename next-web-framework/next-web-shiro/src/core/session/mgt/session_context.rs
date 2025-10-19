

pub trait SessionContext
where 
Self: Send + Sync
{
    fn set_host(&mut self, host: &str);
}