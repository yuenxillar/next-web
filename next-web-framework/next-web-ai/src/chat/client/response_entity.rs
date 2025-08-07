

#[derive(Clone, serde::Deserialize)]
pub struct ResponseEntity<R, E> {
    pub(crate) response: R,
    pub(crate) entity: Option<E>,
}

impl <R, E> ResponseEntity<R, E> {
    
    pub fn new(response: R, entity: Option<E>) -> ResponseEntity<R, E> {
        Self {
            response,
            entity,
        }
    }

    pub fn response(&self) -> &R {
        &self.response
    }

    pub fn entity(&self) -> &Option<E> {
        &self.entity
    }

}