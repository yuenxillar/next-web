use from_attr::FromAttr;
use proc_macro::TokenStream;
#[cfg(feature = "embed-resources")]
use proc_macro2::Span;
use quote::quote;
use syn::ItemFn;
#[cfg(feature = "embed-resources")]
use syn::{spanned::Spanned, Block, Error, LitStr};

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
        add_resources(&mut item_fn.block)?;

        #[cfg(feature = "embed-resources")]
        let resources = resources
            .filter(|s| !s.value().is_empty())
            .unwrap_or(LitStr::new("resources/", Span::call_site()));

        #[cfg(feature = "embed-resources")]
        let token_stream = quote! {

            #[derive(::next_web::embed::Embed, Debug)]
            #[crate_path = "::next_web::embed"]
            #[folder = #resources]
            #[include = "*.html"]
            #[include = "*.json"]
            #[include = "*.xml"]
            #[include = "*.yaml"]
            #[include = "*.yml"]
            #[include = "*.properties"]
            #[include = "*.toml"]
            #[include = "*.txt"]
            pub(crate) struct _ApplicationResources;

            impl ::next_web::context::application_resources::ResourceLoader for _ApplicationResources {

                fn load(&self, path: &str) -> ::std::option::Option<::std::borrow::Cow<'static, [u8]>> {
                    Self::get(path).map(|fs| fs.data)
                }

                fn load_dir(&self, dir: &str) -> ::std::vec::Vec<::std::borrow::Cow<'static, str>> {
                    let mut s = self.iter();
                    s.retain(|s| s.starts_with(dir));

                    s
                }

                fn iter(&self) -> ::std::vec::Vec<::std::borrow::Cow<'static, str>> {
                    Self::iter().collect()
                }
            }

            #item_fn
        };

        Ok(token_stream)
    });

    // println!("expanded: {}", expanded.to_string());

    expanded
}

#[cfg(feature = "embed-resources")]
fn add_resources(block: &mut Box<Block>) -> Result<(), Error> {
    use syn::{Expr, Stmt};

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

    let _stmt = syn::parse_quote! {
        let _ = ::next_web::context::application_resources::RESOURCE_LOADER.set(
            ::std::sync::Arc::new(_ApplicationResources {})
        );

    };
    _async_block.stmts.insert(0, _stmt);

    Ok(())
}
