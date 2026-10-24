use from_attr::FromAttr;
use syn::LitStr;

#[derive(FromAttr)]
#[attribute(idents = [_none])]
pub(crate) struct ApplicationAttr {
    pub resources: Option<LitStr>,
}
