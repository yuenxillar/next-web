/// Represents the scope of the provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    /// singleton scope.
    ///
    /// 1. the constructor run only once.
    /// 2. the type implements [`Clone`] trait.
    /// 3. instances taken from context can be either instances with ownership or reference instances.
    Singleton,
     /// prototype scope.
    ///
    /// 1. the constructor run every time.
    /// 2. instances taken from the context are instances with ownership.
    Prototype
}