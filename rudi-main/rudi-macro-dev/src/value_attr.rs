use from_attr::FromAttr;
use syn::{parse_quote, Expr};

#[derive(FromAttr)]
#[attribute(idents = [value])]
pub(crate) struct ValueAttr {
    #[attribute(default = default_key())]
    pub(crate) key: Expr,
}

fn default_key() -> Expr {
    parse_quote!("")
}

impl Default for ValueAttr {
    fn default() -> Self {
        Self {
            key: default_key(),
        }
    }
}
