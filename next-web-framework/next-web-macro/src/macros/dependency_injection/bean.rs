use from_attr::{ConvertParsed, FromAttr, PathValue};
use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, Expr};
use syn::spanned::Spanned;
use syn::parse_quote;
use syn::Item;

use super::item_fn_gen;


pub fn impl_macro_bean(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = match FunctionAttr::from_tokens(attr.into()) {
        Ok(attr) => attr,
        Err(err) => return err.to_compile_error().into(),
    };

    let item = parse_macro_input!(item as Item);

    let scope = attr.scope.to_token_stream().to_string();
    let result = match item {
        Item::Fn(item_fn) => item_fn_gen::generate(attr, item_fn, scope.into()),
        _ => Err(syn::Error::new(
            item.span(),
            "Only allow function implementation",
        )),
    };
    result.unwrap_or_else(|e| e.to_compile_error()).into()
}



#[derive(FromAttr)]
#[attribute(idents = [autowired])]
pub(super) struct FunctionAttr {
    pub(crate) name: Option<Expr>,
    #[attribute(default = default_scope())]
    pub(crate) scope: Expr,
    pub(crate) condition: Option<ClosureOrPath>,
    #[attribute(default = default_order())]
    pub(crate) order: Expr,
}

fn default_order() -> Expr {
    parse_quote!(i32::MAX)
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