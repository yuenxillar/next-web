use axum::extract::Request;

use crate::util::locale::Locale;

pub trait LocaleResolver {
    fn resolve_locale(&self, req: &mut Request) -> Locale;
}
