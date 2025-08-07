use crate::chat::observation::ai_operation_metadata::AiOperationMetadata;



#[derive(Clone)]
pub struct ModelObservationContext<Q, R> {
    request: Q,
    operation_metadata: AiOperationMetadata,
    response: R,
}

impl<Q, R> ModelObservationContext<Q, R> {
    
    pub fn operation_metadata(&self) -> &AiOperationMetadata {
        &self.operation_metadata
    }

    pub fn request(&self) -> &Q {
        &self.request
    }

    pub fn response(&self) -> &R {
        &self.response
    }
}