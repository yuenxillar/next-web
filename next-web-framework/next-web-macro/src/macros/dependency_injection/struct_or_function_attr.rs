use from_attr::{ConvertParsed, FromAttr, PathValue};
use syn::{parse_quote, spanned::Spanned, Expr, ExprPath};

#[derive(FromAttr)]
#[attribute(idents = [autowired])]
pub(crate) struct StructOrFunctionAttr {
    #[attribute(default = default_name())]
    pub(crate) name: Expr,
    #[attribute(default = default_scope())]
    pub(crate) scope: Expr,

    pub(crate) binds: Vec<ExprPath>,
}

fn default_name() -> Expr {
    parse_quote!("")
}

fn default_scope() -> Expr {
    parse_quote!("Singleton")
}

pub(crate) struct ClosureOrPath(pub(crate) Expr);

impl ConvertParsed for ClosureOrPath {
    type Type = Expr;

    fn convert(path_value: PathValue<Self::Type>) -> syn::Result<Self> {
        let expr = path_value.value;

        match &expr {
            Expr::Closure(_) | Expr::Path(_) => Ok(Self(expr)),
            _ => Err(syn::Error::new(
                expr.span(),
                "the expr must be a closure or an expression path",
            )),
        }
    }
}
