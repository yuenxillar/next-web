use super::observation_registry::{ObservationConfig, ObservationRegistry};

#[derive(Clone)]
pub struct SimpleObservationRegistry {}

impl ObservationRegistry for SimpleObservationRegistry {
    fn current_observation(&self) {
        todo!()
    }

    fn current_observation_scope(&self) {
        todo!()
    }

    fn set_current_observation_scope(&self) {
        todo!()
    }

    fn observation_config(&self) -> &ObservationConfig {
        todo!()
    }

    fn is_noop(&self) -> bool {
        todo!()
    }
}
