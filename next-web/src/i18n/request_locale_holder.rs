use std::future::Future;

use next_web_core::util::locale::Locale;

tokio::task_local! {
    static LOCALE: Locale;
}

/// Before use, you can use # [translation] to store the locality
pub struct RequestLocaleHolder;

impl RequestLocaleHolder {
    pub fn locale() -> Option<Locale> {
        LOCALE.try_get().ok()
    }

    pub async fn scope<F>(locale: Locale, f: F) -> F::Output
    where
        F: Future,
    {
        LOCALE.scope(locale, f).await
    }

    pub fn locale_or_default() -> Locale {
        LOCALE.try_get().unwrap_or_default()
    }

    pub fn locale_with_default(default: Locale) -> Locale {
        LOCALE.try_get().unwrap_or(default)
    }
}
