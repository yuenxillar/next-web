use crate::util::locale::Locale;



pub trait LocaleResolver: Send + Sync {
    
    fn resolve_locale(&self, req: & axum::http::Request<axum::body::Body>) -> Locale;
    
}