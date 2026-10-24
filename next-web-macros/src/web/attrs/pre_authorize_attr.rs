use from_attr::FromAttr;
use syn::LitStr;

/// The arguments of the `#[pre_authorize]` attribute.
///
/// Both a single expression string (`#[pre_authorize("has_role('ADMIN')")]`) and
/// the structured key/value form are supported by
/// [`crate::web::pre_authorize::impl_macro_pre_authorize`]. This struct covers
/// the key/value form.
#[derive(FromAttr)]
#[attribute(idents = [value])]
pub struct PreAuthorizeAttr {
    /// A Spring Security style expression, for example
    /// `"has_role('ADMIN') and has_authority('order:read')"`.
    pub expr: Option<LitStr>,

    /// Grants access when the authenticated user holds the role.
    pub has_role: Option<LitStr>,

    /// Grants access when the authenticated user holds any of the roles.
    pub has_any_role: Option<Vec<LitStr>>,

    /// Grants access when the authenticated user holds the authority.
    pub has_authority: Option<LitStr>,

    /// Grants access when the authenticated user holds any of the authorities.
    pub has_any_authority: Option<Vec<LitStr>>,

    /// Legacy alias for a list of required roles.
    pub role: Option<Vec<LitStr>>,

    /// Legacy alias for a list of required authorities.
    pub permission: Option<Vec<LitStr>>,

    /// How the configured checks are combined: `"And"` (the default) requires
    /// every check to pass, `"Or"` requires at least one of them to pass.
    pub mode: Option<LitStr>,

    /// Inverts the whole rule when set to `true`, which is how a rule such as
    /// "the user must not hold this role" is expressed.
    pub negate: Option<bool>,

    /// Skips the authorization check entirely, emitting the handler unchanged.
    pub ignore: Option<bool>,

    /// Reserved for HTTP basic authorization. Currently rejected at compile
    /// time so a rule is never silently dropped.
    pub basic: Option<LitStr>,
}
