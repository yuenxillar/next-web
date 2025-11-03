use axum::{extract::Request, response::Response};

use crate::core::subject::Subject;


pub trait WebSubject
where 
Self: Subject
{
    fn request(&self) -> &mut Request;

    fn response(&self) -> &mut Response;
}