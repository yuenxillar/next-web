pub mod http_method;
pub mod locale;
pub mod mime_type;
pub mod pattern;
pub mod pattern_match;
pub mod singleton;
pub mod time;

mod string;
mod web;

pub use string::StringUtils;
pub use web::WebUtils;
