pub trait RabbitListener: Send + Sync + 'static {
    fn queue() -> String;
}
