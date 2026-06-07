use std::sync::Arc;

use crate::core::{context::SecurityContext, Authentication};

#[derive(Default)]
pub struct SecurityContextImpl {
    authentication: Option<Arc<dyn Authentication>>,
}

impl SecurityContext for SecurityContextImpl {
    fn get_authentication(&self) -> Option<Arc<dyn Authentication>> {
        todo!()
    }

    fn set_authentication(&self, authentication: Option<Arc<dyn Authentication>>) {
        todo!()
    }
}
