use std::sync::Arc;

use axum::extract::FromRef;

use crate::interface::data_decoder::DataDecoder;


#[derive(Clone)]
pub struct AppState {
    data_decoder: Option<Arc<dyn DataDecoder>>,
}


impl AppState {
    
    pub fn new(data_decoder: Option<Arc<dyn DataDecoder>>) -> Self {
        Self {
            data_decoder,
        }
    }

    pub fn data_decoder(&self) -> Option<&Arc<dyn DataDecoder>> {
        self.data_decoder.as_ref()
    }
}


impl FromRef<AppState> for Option<Arc<dyn DataDecoder>> {
    fn from_ref(state: &AppState) -> Self {
        state.data_decoder.clone()
    }
}