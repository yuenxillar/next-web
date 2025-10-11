use crate::core::filter::Filter;


pub struct LogoutFilter {}

impl Filter for LogoutFilter {
    fn do_filter(&self, req: & axum::extract::Request, res: & axum::response::Response, next: axum::middleware::Next) {
        todo!()
    }
}