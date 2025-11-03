

pub trait Authentication
where Self: Send + Sync
{

    fn is_authenticated(&self) -> bool;
    
}