use from_attr::FromAttr;
use syn::{parse_quote, Expr};

#[derive(FromAttr)]
#[attribute(idents = [value])]
pub(crate) struct ValueAttr {
    #[attribute(default = default_key())]
    pub(crate) key: Expr,

    /// The value the field takes when the property is not configured at all.
    ///
    /// `None` means [`Default::default`], so `#[value(key = "…")]` on its own
    /// never fails for a property that is simply absent.
    #[attribute(default)]
    pub(crate) default: Option<Expr>,
}

fn default_key() -> Expr {
    parse_quote!("")
}

impl Default for ValueAttr {
    fn default() -> Self {
        Self {
            key: default_key(),
            default: None,
        }
    }
}
