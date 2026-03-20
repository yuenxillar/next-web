use from_attr::FromAttr;
use syn::{ExprPath, LitInt, LitStr};

#[derive(FromAttr)]
#[attribute(idents = [provider])]
pub(crate) struct ProviderAttr {
    pub name: Option<LitStr>,
    pub conditional: Option<Vec<ExprPath>>,
    pub order: Option<LitInt>,
}

#[derive(FromAttr)]
#[attribute(idents = [conditional_on_property])]
pub(crate) struct ConditionalOnPropertyAttr {
    pub name: LitStr,
    pub having_value: LitStr,
}

#[derive(FromAttr)]
#[attribute(idents = [autowired])]
pub(crate) struct AutowiredAttr {
    pub name: Option<LitStr>,
    pub default: Option<bool>,
}
