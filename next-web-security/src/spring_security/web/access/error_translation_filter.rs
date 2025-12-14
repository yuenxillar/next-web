use next_web_core::error::BoxError;

use crate::core::filter::Filter;



pub struct ErrorTranslationFilter {}


impl Filter for ErrorTranslationFilter {
    fn do_filter(&self, req: &mut axum::extract::Request, res: &mut axum::response::Response) -> Result<(), BoxError>{
        todo!()
    }
}