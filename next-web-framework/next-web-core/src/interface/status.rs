pub trait ServiceStatus: Send + Sync {
    fn status(&self) -> ();
}
