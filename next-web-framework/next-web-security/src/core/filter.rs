use axum::{extract::Request, middleware::Next, response::Response};



pub trait Filter: Send + Sync {
    
    fn do_filter(&self, req: & Request, res: & Response, next: Next);

    fn destory(&mut self) {} 
}