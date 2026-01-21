use crate::util::locale::Locale;

pub trait LocaleResolver {
    fn resolve_locale(req: &axum::extract::Request) -> Locale;
}
