use crate::observation::observation::Observation;


pub struct NoopObservation {}


impl NoopObservation {
}

impl Default for NoopObservation {
    fn default() -> Self {
        Self {  }
    }
}

impl Observation for  NoopObservation {
    fn start(&mut self) {
        todo!()
    }

    fn context(&mut self) -> &mut dyn super::observation::Context {
        todo!()
    }

    fn stop(&mut self) {
        todo!()
    }

    fn contextual_name(&mut self, contextual_name: &str) {
        todo!()
    }
    
    fn parent_observation(&mut self, parent_observation: Box<dyn Observation>) {
        todo!()
    }
    
    fn low_cardinality_key_value(&mut self, key_value: Box<dyn crate::util::key_value::KeyValue>) {
        todo!()
    }
    
    fn high_cardinality_key_value(&mut self, key_value: Box<dyn crate::util::key_value::KeyValue>) {
        todo!()
    }
    
    fn observation_convention(&mut self, observation_convention: super::observation_documentation::BoxObservationConvention) {
        todo!()
    }
    
    fn error(&mut self, error: & next_web_core::error::BoxError) {
        todo!()
    }
    
    fn event(&mut self, event: Box<dyn super::observation::Event>) {
        todo!()
    }
    
    fn open_scope(&self) -> Box<dyn super::observation::Scope> {
        todo!()
    }
}