use from_attr::FromAttr;
use proc_macro::TokenStream;
#[cfg(feature = "embed-resources")]
use proc_macro2::Span;
use quote::quote;
use syn::ItemFn;
#[cfg(feature = "embed-resources")]
use syn::{spanned::Spanned, Error, LitStr};

use crate::{util::logic::Logic, web::attrs::application_attr::ApplicationAttr};

pub fn impl_macro_application(
    attrs: TokenStream,
    #[allow(unused_mut)] mut item_fn: ItemFn,
) -> TokenStream {
    let expanded = Logic::generate(move || {
        #[allow(unused_variables)]
        let ApplicationAttr { resources } = match ApplicationAttr::from_tokens(attrs.into()) {
            Ok(attr) => attr,
            Err(error) => return Err(error),
        };

        #[cfg(not(feature = "embed-resources"))]
        let token_stream = quote! {
            #item_fn
        };

        #[cfg(feature = "embed-resources")]
        add_resources(&mut item_fn)?;

        #[cfg(feature = "embed-resources")]
        let resources = resources
            .filter(|s| !s.value().is_empty())
            .unwrap_or(LitStr::new("resources/", Span::call_site()));

        #[cfg(feature = "embed-resources")]
        let token_stream = quote! {
            #[derive(::next_web::embed::Embed, Debug)]
            #[crate_path = "::next_web::embed"]
            #[folder = #resources]
            pub(crate) struct EmbedResourceLoader;

            #item_fn
        };

        Ok(token_stream)
    });

    // println!("expanded: {}", expanded.to_string());

    expanded
}

#[cfg(feature = "embed-resources")]
fn add_resources(item_fn: &mut ItemFn) -> Result<(), Error> {
    use syn::{Expr, Stmt};

    let install = syn::parse_quote! {
        ::next_web::resources::install_embedded_resources::<EmbedResourceLoader>();
    };

    // The attribute is used above `#[tokio::main]` in this case, so the
    // function still is the async function the user wrote.
    if item_fn.sig.asyncness.is_some() {
        item_fn.block.stmts.insert(0, install);
        return Ok(());
    }

    // `#[tokio::main]` was applied first: it turned the async function into a
    // synchronous wrapper whose body starts with `let body = async move {...};`.
    // The install statement is inserted at the start of that async block.
    let block = &mut item_fn.block;
    if block.stmts.is_empty() {
        return Err(Error::new(
            block.span(),
            "Function body is empty, expected async block",
        ));
    }

    let expr = match block.stmts.first_mut() {
        Some(stmt) => match stmt {
            Stmt::Local(local) => match local.init.as_mut() {
                Some(init) => init.expr.as_mut(),
                None => return Err(Error::new(local.span(), "")),
            },
            _ => return Err(Error::new(stmt.span(), "")),
        },
        None => return Err(Error::new(block.span(), "")),
    };

    let _async_block = match expr {
        Expr::Async(expr_async) => &mut expr_async.block,
        _ => return Err(Error::new(expr.span(), "")),
    };

    _async_block.stmts.insert(0, install);

    Ok(())
}
