use crate::core::filter::Filter;



pub struct ErrorTranslationFilter {}


impl Filter for ErrorTranslationFilter {
    fn do_filter(&self, req: & axum::extract::Request, res: & axum::response::Response, next: axum::middleware::Next) {
        todo!()
    }
}