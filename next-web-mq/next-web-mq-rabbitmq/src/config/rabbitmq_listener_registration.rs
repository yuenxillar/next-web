/// Fluent registration contract for RabbitMQ listener endpoints.
pub trait RabbitmqListenerRegistration {
    fn consumer_tag(&mut self, consumer_tag: String) -> &mut dyn RabbitmqListenerRegistration;

    fn exchange_type(&mut self, exchange_type: String) -> &mut dyn RabbitmqListenerRegistration;

    fn exchange_durable(&mut self, durable: bool) -> &mut dyn RabbitmqListenerRegistration;

    fn queue_durable(&mut self, durable: bool) -> &mut dyn RabbitmqListenerRegistration;
}
