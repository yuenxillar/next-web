pub trait Authentication
where
    Self: Send + Sync,
{
    fn is_authenticated(&self) -> bool;

    fn authorities(&self) -> Vec<String> {
        Vec::new()
    }

    fn is_anonymous(&self) -> bool {
        false
    }

    fn is_remember_me(&self) -> bool {
        false
    }
}
