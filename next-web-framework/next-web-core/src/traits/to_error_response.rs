use axum::response::Response;

pub trait ToErrorResponse
where 
Self : Send + Sync
{
    fn to_error_response(&self, error_message: Option<String>) -> Response;
}