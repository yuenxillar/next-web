use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};

use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    GenericArgument, Ident, ItemImpl, LitStr, PathArguments, Type, TypePath,
};

pub fn impl_macro_event_listener(args: TokenStream, input: ItemImpl) -> TokenStream {
    let args = if args.is_empty() {
        EventListenerArgs { id: None }
    } else {
        parse_macro_input!(args as EventListenerArgs)
    };

    let listener_type = &input.self_ty;

    let listener_name = match &**listener_type {
        Type::Path(TypePath { path, .. }) => path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_else(|| "UnknownListener".to_string()),
        _ => "UnknownListener".to_string(),
    };

    let id = args
        .id
        .as_ref()
        .map(|s| s.trim_end().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(listener_name.clone());
    let has_id = args
        .id
        .as_ref()
        .map(|s| !s.trim_end().is_empty())
        .unwrap_or_default();
    let listener_id = LitStr::new(&id, Span::call_site());

    let impl_id = if has_id {
        quote! {
            impl ::next_web::core::traits::id::Id for #listener_type {
                fn id(&self) -> &'static str { #listener_id }
            }
        }
    } else {
        quote! {}
    };

    // Extract trait paths and generic parameters
    let event =
        extract_event_type_from_impl(&input).expect("Failed to extract event type from impl block");

    let impl_add_listener = if has_id {
        quote! { multicaster.add_application_listener::<#listener_type, #event>(#listener_id.into(), listener).await; }
    } else {
        quote! { multicaster.add_application_listener::<#listener_type, #event>(listener.id().into(), listener).await; }
    };

    // Create a new identifier for registering the structure
    let name = format_ident!(
        "__auto_register_{}",
        listener_name,
        span = listener_type.span()
    );

    // Generate code
    let expanded = quote! {
        #input

        #impl_id

        #[allow(non_camel_case_types)]
        pub(crate) struct #name;

        impl ::next_web::autoregister::application_event_autoregister::ApplicationEventAutoRegister  for  #name {
            fn register<'life_a>(&'life_a self,
                ctx: &'life_a mut ::next_web::core::ApplicationContext,
                multicaster: &'life_a mut ::next_web::event::default_application_event_multicaster::DefaultApplicationEventMulticaster
            ) -> ::core::pin::Pin<::std::boxed::Box<dyn ::core::future::Future<Output = ()> + Send + 'life_a>>
            {
                ::std::boxed::Box::pin(async move {
                    use ::next_web::core::traits::event::application_event_multicaster::ApplicationEventMulticaster;

                    let listener = ctx.resolve_with_default_name::<#listener_type>();
                    #impl_add_listener
                })
            }
        }

        ::next_web::submit_application_event_autoregister!(#name);
    };

    // println!("{}", expanded.to_string());

    TokenStream::from(expanded)
}

struct EventListenerArgs {
    id: Option<String>,
}

impl Parse for EventListenerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut id = None;

        // 解析属性参数，格式: id = "value"
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            if ident == "id" {
                input.parse::<syn::Token![=]>()?;
                let value: LitStr = input.parse()?;
                id = Some(value.value());
            } else {
                return Err(syn::Error::new(ident.span(), "Unknown attribute"));
            }

            // 如果有逗号，则继续解析
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(EventListenerArgs { id })
    }
}

fn extract_event_type_from_impl(input: &ItemImpl) -> Option<&Type> {
    let (_, path, _) = input.trait_.as_ref()?;
    let segment = path.segments.last()?;

    match &segment.arguments {
        PathArguments::AngleBracketed(args) => {
            if let Some(GenericArgument::Type(event_ty)) = args.args.first() {
                Some(event_ty)
            } else {
                None
            }
        }
        _ => None,
    }
}
