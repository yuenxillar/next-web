use from_attr::FromAttr;
use syn::parse_quote;
use syn::Expr;

#[derive(FromAttr)]
#[attribute(idents = [value])]
pub(crate) struct PropertiesAttr {
    #[attribute(default = default_name())]
    pub(crate) prefix: Expr,

    pub(crate) dynamic: bool,
}

fn default_name() -> Expr {
    parse_quote!()
}

impl Default for PropertiesAttr {
    fn default() -> Self {
        Self {
            prefix: default_name(),
            dynamic: false,
        }
    }
}