use from_attr::{FlagOrValue, FromAttr};
use syn::{parse_quote, Expr, Type};

// #[autowired(
//     name = "testService",
//     default = 42,
// )]

#[derive(FromAttr)]
#[attribute(idents = [autowired])]
pub(crate) struct FieldOrArgumentAttr {
    #[attribute(default = default_name(), conflicts = [vec])]
    pub(crate) name: Expr,

    #[attribute(conflicts = [option, vec])]
    pub(crate) default: FlagOrValue<Expr>,

    #[attribute(rename = "ref")]
    pub(crate) ref_: FlagOrValue<Type>,
}

fn default_name() -> Expr {
    parse_quote!("")
}

impl Default for FieldOrArgumentAttr {
    fn default() -> Self {
        Self {
            name: default_name(),
            default: Default::default(),
            ref_: Default::default(),
        }
    }
}
