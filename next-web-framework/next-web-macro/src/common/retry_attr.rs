use std::path::Path;

use from_attr::FromAttr;
use syn::{parse_quote, Expr, ExprPath, Lit};

#[derive(FromAttr)]
#[attribute(idents = [value])]
pub(crate) struct RetryAttr {

    #[attribute(default = default_max_attempts())]
    pub(crate) max_attempts: Expr,

    #[attribute(default = default_delay())]
    pub(crate) delay: Expr,

    pub(crate) backoff: Option<ExprPath>,

    pub(crate) retry_for: Option<Expr>
}

fn default_max_attempts() -> Expr {
    parse_quote!(1)
}

fn default_delay() -> Expr {
    parse_quote!(1000)
}


impl Default for RetryAttr {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            delay: default_delay(),
            backoff: None,
            retry_for: None
        }
    }
}
