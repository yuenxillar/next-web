use crate::env::{ParseError, ProfilesParser};

/// A profile predicate that may be accepted by an [`Environment`].
///
/// A `Profiles` instance encapsulates a condition over the currently active
/// profiles. It does not know which profiles are active; instead, the caller
/// supplies a predicate that answers that question. This inversion of control
/// keeps `Profiles` independent of any concrete [`Environment`] implementation
/// and makes it trivial to test.
///
/// Instances are usually created through the [`Profiles::of`] factory method
/// rather than implemented directly.
///
/// # Examples
///
/// ```
/// use std::collections::HashSet;
///
/// let active: HashSet<&str> = ["dev", "cloud"].into_iter().collect();
/// let is_active = |name: &str| active.contains(name);
///
/// assert!(Profiles::of(&["dev"]).matches(&is_active));
/// assert!(Profiles::of(&["production & cloud"]).matches(&is_active));
/// assert!(!Profiles::of(&["production"]).matches(&is_active));
/// ```
///
/// [`Environment`]: crate::env::Environment
pub trait Profiles {
    /// Test whether this instance matches against the given predicate.
    ///
    /// The `is_profile_active` predicate reports whether a profile name is
    /// currently active. Implementations may call it lazily and short-circuit
    /// evaluation of compound expressions.
    fn matches(&self, is_profile_active: &dyn Fn(&str) -> bool) -> bool;
}

/// Create a new [`Profiles`] instance that matches against the given
/// profile expressions.
///
/// The returned instance matches if **any** of the given expressions
/// matches.
///
/// A profile expression is either a simple profile name (for example
/// `"production"`) or a compound expression (for example
/// `"production & cloud"`).
///
/// The following operators are supported:
///
/// - `!` — logical NOT of a profile name or compound expression
/// - `&` — logical AND of profile names or compound expressions
/// - `|` — logical OR of profile names or compound expressions
///
/// The `&` and `|` operators may not be mixed without parentheses.
/// For example, `"a & b | c"` is not a valid expression: it must be
/// written as `"(a & b) | c"` or `"a & (b | c)"`.
///
/// Two `Profiles` instances created from identical profile expressions
/// are considered equal and produce the same hash.
///
/// # Errors
///
/// Returns an error if any expression is malformed.
pub fn profiles_of(profile_expressions: &[&str]) -> Result<Box<dyn Profiles>, ParseError> {
    ProfilesParser::parse(profile_expressions)
}
