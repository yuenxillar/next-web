use from_attr::FromAttr;
use syn::LitStr;

#[derive(FromAttr)]
#[attribute(idents = [value])]
pub struct PreAuthorizeAttr {
    pub role: Option<Vec<LitStr>>,
    pub permission: Option<Vec<LitStr>>,
    // Or And
    pub mode: Option<LitStr>,

    #[attribute(conflicts = [role, permission, mode, basic])]
    pub ignore: Option<bool>,

    // http basic auth
    #[attribute(conflicts = [ignore, role, permission, mode])]
    pub basic: Option<LitStr>,
}