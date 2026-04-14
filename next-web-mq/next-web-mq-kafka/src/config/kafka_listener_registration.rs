/// Fluent registration contract for Kafka listener endpoints.
pub trait KafkaListenerRegistration {
    fn group_id(&mut self, group_id: String) -> &mut dyn KafkaListenerRegistration;

    fn client_id_prefix(&mut self, client_id_prefix: String) -> &mut dyn KafkaListenerRegistration;

    fn concurrency(&mut self, concurrency: u16) -> &mut dyn KafkaListenerRegistration;

    fn auto_startup(&mut self, auto_startup: bool) -> &mut dyn KafkaListenerRegistration;

    fn poll_timeout_ms(&mut self, poll_timeout_ms: u64) -> &mut dyn KafkaListenerRegistration;
}
