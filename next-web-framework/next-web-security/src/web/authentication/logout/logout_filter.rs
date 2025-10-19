use crate::core::filter::Filter;


pub struct LogoutFilter {}

impl Filter for LogoutFilter {
    fn do_filter(&self, req: &mut axum::extract::Request, res: &mut axum::response::Response) -> Result<(), next_web_core::error::BoxError>{
        todo!()
    }
}