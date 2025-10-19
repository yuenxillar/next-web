use super::Session;


pub struct ProxiedSession {

}

impl Session for ProxiedSession {
    fn id(&self) -> super::SessionId {
        todo!()
    }

    fn start_timestamp(&self) -> std::time::SystemTime {
        todo!()
    }

    fn last_access_time(&self) -> std::time::SystemTime {
        todo!()
    }

    fn timeout(&self) -> Result<u64, super::InvalidSessionError> {
        todo!()
    }

    fn set_timeout(&mut self, max_idle_time_in_millis: u64) -> Result<(), super::InvalidSessionError> {
        todo!()
    }

    fn host(&self) -> Option<&str> {
        todo!()
    }

    fn touch(&mut self) -> Result<(), super::InvalidSessionError> {
        todo!()
    }

    fn stop(&mut self) -> Result<(), super::InvalidSessionError> {
        todo!()
    }

    fn attribute_keys(&self) -> Result<std::collections::HashSet<String>, super::InvalidSessionError> {
        todo!()
    }

    fn get_attribute(&self, key: &str) -> Result<Option<super::SessionValue>, super::InvalidSessionError> {
        todo!()
    }

    fn set_attribute(
        &mut self,
        key: &str,
        value: Option<super::SessionValue>,
    ) -> Result<(), super::InvalidSessionError> {
        todo!()
    }

    fn remove_attribute(&mut self, key: &str) -> Result<Option<super::SessionValue>, super::InvalidSessionError> {
        todo!()
    }
}