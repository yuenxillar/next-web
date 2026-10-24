//! Port of `org.springframework.context.i18n.LocaleContextHolder`.
//!
//! The holder associates a [`LocaleContext`] with the code that is currently
//! running, and it answers the locale the framework resolves without an explicit
//! locale, for example when a message is looked up.
//!
//! The original stores the context in a thread local, together with an
//! inheritable one for the threads the current thread spawns. A task of an async
//! runtime can move to another thread between two awaits, so this framework
//! stores the context in a task local as well:
//!
//! - [`set_locale_context`](LocaleContextHolder::set_locale_context) associates
//!   the context with the current thread, which is what the original does and
//!   what synchronous code needs;
//! - [`scope`](LocaleContextHolder::scope) associates the context with the
//!   current task for the duration of a future, which is what asynchronous code
//!   needs.
//!
//! Reading the context looks the task local up first and the thread local
//! second. A holder without a context falls back to the
//! [shared default locale](LocaleContextHolder::set_default_locale) and then to
//! the locale of the system, so it is a replacement for the default locale of
//! the platform that can respect an application-level setting.
//!
//! The holder of this framework knows no time zones: the original can associate
//! a time zone with the context as well, but this framework has no time-zone
//! context, so only the locale part of a context is held.

use std::cell::RefCell;
use std::future::Future;
use std::sync::{Arc, OnceLock};

use arc_swap::ArcSwapOption;

use crate::Locale;
use crate::i18n::LocaleContext;
use crate::i18n::SimpleLocaleContext;

tokio::task_local! {
    /// The locale context of the current task.
    static TASK_LOCALE_CONTEXT: Arc<dyn LocaleContext>;
}

thread_local! {
    /// The locale context of the current thread.
    static THREAD_LOCALE_CONTEXT: RefCell<Option<Arc<dyn LocaleContext>>> =
        const { RefCell::new(None) };
}

/// The shared default locale of the framework.
static DEFAULT_LOCALE: OnceLock<ArcSwapOption<Locale>> = OnceLock::new();

/// Associates a [`LocaleContext`] with the code that is currently running.
///
/// The type is the counterpart of `LocaleContextHolder`, and it is used wherever
/// the framework resolves a locale without being given one, such as by a message
/// source accessor.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocaleContextHolder;

impl LocaleContextHolder {
    /// Resets the locale context of the current thread.
    ///
    /// Equivalent to `resetLocaleContext()`. A locale context that was given to
    /// [`scope`](Self::scope) belongs to the future it was given to and is
    /// therefore reset when that future is dropped.
    pub fn reset_locale_context() {
        THREAD_LOCALE_CONTEXT.with(|locale_context| *locale_context.borrow_mut() = None);
    }

    /// Associates the given locale context with the current thread.
    ///
    /// Equivalent to `setLocaleContext(LocaleContext)`: a locale context of
    /// `None` resets the context of the thread.
    ///
    /// Asynchronous code should use [`scope`](Self::scope) instead, since a task
    /// can move to another thread between two awaits.
    ///
    /// # Arguments
    ///
    /// * `locale_context` - The context to associate, or `None` to reset.
    pub fn set_locale_context(locale_context: Option<Arc<dyn LocaleContext>>) {
        THREAD_LOCALE_CONTEXT.with(|current| *current.borrow_mut() = locale_context);
    }

    /// Returns the locale context of the current task or thread.
    ///
    /// Equivalent to `getLocaleContext()`.
    ///
    /// # Returns
    ///
    /// The context that is associated with the current task, then the one that
    /// is associated with the current thread, or `None` when there is none.
    pub fn locale_context() -> Option<Arc<dyn LocaleContext>> {
        if let Ok(locale_context) = TASK_LOCALE_CONTEXT.try_with(Arc::clone) {
            return Some(locale_context);
        }

        THREAD_LOCALE_CONTEXT.with(|locale_context| locale_context.borrow().clone())
    }

    /// Associates the given locale with the current thread.
    ///
    /// Equivalent to `setLocale(Locale)`: the locale becomes a
    /// [`SimpleLocaleContext`], and `None` resets the context of the thread.
    ///
    /// The original preserves the time zone of the context it replaces, which
    /// this framework does not hold.
    ///
    /// # Arguments
    ///
    /// * `locale` - The locale to associate, or `None` to reset.
    pub fn set_locale(locale: Option<Locale>) {
        Self::set_locale_context(
            locale
                .map(|locale| Arc::new(SimpleLocaleContext::new(locale)) as Arc<dyn LocaleContext>),
        );
    }

    /// Returns the locale of the current context.
    ///
    /// Equivalent to `getLocale()`: the locale of the context of the current
    /// task or thread wins, then the
    /// [shared default locale](Self::set_default_locale), then the locale of the
    /// system.
    pub fn locale() -> Locale {
        Self::locale_of(Self::locale_context().as_deref())
    }

    /// Returns the locale of the given context.
    ///
    /// Equivalent to `getLocale(LocaleContext)`.
    ///
    /// # Arguments
    ///
    /// * `locale_context` - The user-level context to read the locale from, if
    ///   any.
    pub fn locale_of(locale_context: Option<&dyn LocaleContext>) -> Locale {
        locale_context
            .and_then(|locale_context| locale_context.locale())
            .or_else(Self::default_locale)
            .unwrap_or_else(Locale::system_locale)
    }

    /// Sets the shared default locale of the framework.
    ///
    /// Equivalent to `setDefaultLocale(Locale)`: the locale answers the lookups
    /// no context has a locale for, which lets an application declare a default
    /// locale that differs from the locale of the system. Passing `None` makes
    /// those lookups fall back to the locale of the system, which is the
    /// default.
    ///
    /// # Arguments
    ///
    /// * `locale` - The default locale, or `None` for the locale of the system.
    pub fn set_default_locale(locale: Option<Locale>) {
        default_locale_store().store(locale.map(Arc::new));
    }

    /// Returns the shared default locale of the framework.
    ///
    /// # Returns
    ///
    /// The locale set with [`set_default_locale`](Self::set_default_locale), or
    /// `None` when the lookups fall back to the locale of the system.
    pub fn default_locale() -> Option<Locale> {
        default_locale_store()
            .load_full()
            .map(|locale| (*locale).clone())
    }

    /// Runs the given future with the given locale context associated with it.
    ///
    /// The context is the counterpart of the context a thread inherits from the
    /// thread that spawned it: an async task does not inherit the context of its
    /// caller, so a caller hands it over with this function. A context of `None`
    /// runs the future without changing the context of the task.
    ///
    /// # Arguments
    ///
    /// * `locale_context` - The context of the future, if any.
    /// * `future` - The future the context is associated with.
    pub async fn scope<F>(locale_context: Option<Arc<dyn LocaleContext>>, future: F) -> F::Output
    where
        F: Future,
    {
        match locale_context {
            Some(locale_context) => TASK_LOCALE_CONTEXT.scope(locale_context, future).await,
            None => future.await,
        }
    }

    /// Runs the given future with the given locale associated with it.
    ///
    /// # Arguments
    ///
    /// * `locale` - The locale of the future.
    /// * `future` - The future the locale is associated with.
    pub async fn scope_locale<F>(locale: Locale, future: F) -> F::Output
    where
        F: Future,
    {
        Self::scope(
            Some(Arc::new(SimpleLocaleContext::new(locale)) as Arc<dyn LocaleContext>),
            future,
        )
        .await
    }
}

/// Returns the storage of the shared default locale, creating it the first time
/// it is used.
///
/// The storage is read on every lookup, so it is exchanged without a lock
/// instead of being written behind one.
fn default_locale_store() -> &'static ArcSwapOption<Locale> {
    DEFAULT_LOCALE.get_or_init(ArcSwapOption::empty)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Serializes the tests that read or write the shared default locale, since
    /// it belongs to the process and not to a test.
    static SHARED_DEFAULT: Mutex<()> = Mutex::new(());

    /// Returns the locale of the given language tag.
    fn locale(tag: &str) -> Locale {
        Locale::for_language_tag(tag).expect("the test locale is valid")
    }

    /// A context that holds no locale at all.
    #[derive(Debug)]
    struct EmptyLocaleContext;

    impl LocaleContext for EmptyLocaleContext {
        fn locale(&self) -> Option<Locale> {
            None
        }
    }

    #[test]
    fn the_thread_locale_context_is_used_and_reset() {
        let _shared_default = SHARED_DEFAULT
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        LocaleContextHolder::set_locale(Some(locale("de-DE")));
        assert_eq!(LocaleContextHolder::locale(), locale("de-DE"));
        assert!(LocaleContextHolder::locale_context().is_some());

        LocaleContextHolder::reset_locale_context();
        assert!(LocaleContextHolder::locale_context().is_none());
        assert_eq!(LocaleContextHolder::locale(), Locale::system_locale());
    }

    #[test]
    fn a_context_without_a_locale_is_answered_with_the_default_locale() {
        let _shared_default = SHARED_DEFAULT
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        LocaleContextHolder::set_default_locale(Some(locale("zh-CN")));
        assert_eq!(LocaleContextHolder::default_locale(), Some(locale("zh-CN")));

        let empty = EmptyLocaleContext;
        assert_eq!(
            LocaleContextHolder::locale_of(Some(&empty)),
            locale("zh-CN")
        );
        assert_eq!(LocaleContextHolder::locale_of(None), locale("zh-CN"));

        LocaleContextHolder::set_default_locale(None);
        assert_eq!(LocaleContextHolder::default_locale(), None);
        assert_eq!(
            LocaleContextHolder::locale_of(None),
            Locale::system_locale()
        );
    }

    #[test]
    fn the_locale_of_a_context_wins_over_the_default_locale() {
        let _shared_default = SHARED_DEFAULT
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        LocaleContextHolder::set_default_locale(Some(locale("zh-CN")));
        let simple = SimpleLocaleContext::new(locale("ja-JP"));

        assert_eq!(
            LocaleContextHolder::locale_of(Some(&simple)),
            locale("ja-JP")
        );

        LocaleContextHolder::set_default_locale(None);
    }

    #[tokio::test]
    async fn the_task_locale_context_wins_over_the_thread_locale_context() {
        let _shared_default = SHARED_DEFAULT
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        LocaleContextHolder::set_locale(Some(locale("de-DE")));

        LocaleContextHolder::scope_locale(locale("ja-JP"), async {
            assert_eq!(LocaleContextHolder::locale(), locale("ja-JP"));
        })
        .await;

        // The context of the task does not leak into the thread that ran it.
        assert_eq!(LocaleContextHolder::locale(), locale("de-DE"));

        LocaleContextHolder::reset_locale_context();
        assert_eq!(LocaleContextHolder::locale(), Locale::system_locale());
    }

    #[tokio::test]
    async fn a_scoped_context_answers_a_lookup_without_a_thread_context() {
        let _shared_default = SHARED_DEFAULT
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        LocaleContextHolder::reset_locale_context();
        let context: Arc<dyn LocaleContext> = Arc::new(SimpleLocaleContext::new(locale("en-GB")));

        LocaleContextHolder::scope(Some(context), async {
            assert_eq!(LocaleContextHolder::locale(), locale("en-GB"));
        })
        .await;

        assert_eq!(LocaleContextHolder::locale(), Locale::system_locale());
    }
}
