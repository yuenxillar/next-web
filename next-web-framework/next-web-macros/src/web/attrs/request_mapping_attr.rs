use from_attr::FromAttr;
use syn::{parse_quote, Expr, LitStr};

#[derive(FromAttr)]
#[attribute(idents = [find])]
pub(crate) struct RequestMappingAttr {
    #[attribute(rename = "method")]
    pub(crate) method_: Option<LitStr>,
    pub(crate) path: Expr,
    pub(crate) headers: Vec<LitStr>,
    pub(crate) consumes: Option<LitStr>,
    pub(crate) produces: Option<LitStr>,
}

fn default_path() -> Expr {
    parse_quote!("")
}

fn default_list() -> Vec<LitStr> {
    vec![]
}

impl Default for RequestMappingAttr {
    fn default() -> Self {
        Self {
            method_: Default::default(),
            path: default_path(),
            headers: default_list(),
            consumes: Default::default(),
            produces: Default::default(),
        }
    }
}