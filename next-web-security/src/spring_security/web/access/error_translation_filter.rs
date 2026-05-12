use next_web_core::error::BoxError;

use crate::core::filter::Filter;

pub struct ErrorTranslationFilter {}

impl Filter for ErrorTranslationFilter {
    fn do_filter(
        &self,
        _req: &mut axum::extract::Request,
        _res: &mut axum::response::Response,
    ) -> Result<(), BoxError> {
        Ok(())
    }
}
