use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{spanned::Spanned, Data, DeriveInput, Field, Ident, Path};

use crate::util::field_type::FieldType;

/// Implements `Desensitize` for the fields of a structure that name a
/// desensitizer with `#[de]`.
pub(crate) fn impl_macro_desensitize(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => {
            return syn::Error::new_spanned(
                name,
                "#[derive(Desensitize)] is only supported for structs",
            )
            .to_compile_error()
            .into();
        }
    };

    let mut bodies = Vec::new();
    let mut uses_built_in = false;
    let mut errors: Option<syn::Error> = None;

    for field in fields.iter() {
        // Only a named field can be masked, because the generated code writes it
        // by name, and a field the application marked as private is left to it.
        let Some(field_name) = field.ident.as_ref() else {
            continue;
        };
        if field_name.to_string().starts_with('_') || !has_de_attribute(field) {
            continue;
        }

        match desensitizer_of(field).and_then(|desensitizer| match desensitizer {
            Some(desensitizer) => body_of(field, field_name, &desensitizer)
                .map(|body| (body, desensitizer.is_built_in())),
            None => Ok((TokenStream2::new(), false)),
        }) {
            Ok((body, is_built_in)) => {
                uses_built_in |= is_built_in;
                bodies.push(body);
            }
            Err(error) => push_error(&mut errors, error),
        }
    }

    if let Some(error) = errors {
        return error.to_compile_error().into();
    }

    // The types of the framework are named through the util, and an application
    // that only uses functions of its own does not need it.
    let built_in_use = if uses_built_in {
        quote! { use ::next_web::util::desensitized::DesensitizedUtil; }
    } else {
        TokenStream2::new()
    };

    quote! {
        impl ::next_web_core::util::Desensitize for #name {
            fn desensitize(&mut self) {
                #built_in_use

                #(#bodies;)*
            }
        }
    }
    .into()
}

/// Returns whether the field names a desensitizer with `#[de]`.
fn has_de_attribute(field: &Field) -> bool {
    field.attrs.iter().any(|attr| attr.path().is_ident("de"))
}

/// Returns the desensitizer of a field, which its `#[de]` attribute names.
fn desensitizer_of(field: &Field) -> Result<Option<Desensitizer>, syn::Error> {
    let mut desensitizer = None;

    for attr in field.attrs.iter().filter(|attr| attr.path().is_ident("de")) {
        if desensitizer.is_some() {
            return Err(syn::Error::new_spanned(
                attr,
                "a field names its desensitizer once: either a type (`#[de(phone)]`) \
                 or a function (`#[de(with = de_phone)]`)",
            ));
        }

        desensitizer = Some(parse_de_attribute(attr)?);
    }

    Ok(desensitizer)
}

/// Reads the desensitizer a single `#[de]` attribute names.
///
/// The attribute names either a type the framework provides, as in
/// `#[de(phone)]`, or a function of the application, as in
/// `#[de(with = de_phone)]`. The two are mutually exclusive: an attribute that
/// names both is refused, and so is a second attribute on the same field.
fn parse_de_attribute(attr: &syn::Attribute) -> Result<Desensitizer, syn::Error> {
    let mut built_in: Option<DesensitizationType> = None;
    let mut function: Option<Path> = None;

    attr.parse_nested_meta(|meta| {
        // `with = <path>` names a function of the application.
        if meta.path.is_ident("with") {
            if function.is_some() {
                return Err(meta.error("`with` is given once"));
            }

            function = Some(meta.value()?.parse()?);

            return Ok(());
        }

        // A single segment names one of the types the framework provides, and is
        // a function of the application when it names none of them.
        if let Some(ident) = meta.path.get_ident() {
            if let Some(desensitization_type) = DesensitizationType::from_name(&ident.to_string()) {
                if built_in.is_some() {
                    return Err(meta.error("a `de` attribute names one desensitization type"));
                }

                built_in = Some(desensitization_type);

                return Ok(());
            }
        }

        // Any other path is a function of the application.
        if function.is_some() {
            return Err(meta.error("`de` names one desensitizer"));
        }

        function = Some(meta.path.clone());

        Ok(())
    })?;

    match (built_in, function) {
        (Some(desensitization_type), None) => Ok(Desensitizer::BuiltIn(desensitization_type)),
        (None, Some(path)) => Ok(Desensitizer::Function(path)),
        (Some(_), Some(_)) => Err(syn::Error::new_spanned(
            attr,
            "a `de` attribute names either a type (`#[de(phone)]`) or a function \
             (`#[de(with = de_phone)]`), not both",
        )),
        (None, None) => Err(syn::Error::new_spanned(
            attr,
            format!(
                "a `de` attribute names a type ({}) or a function \
                 (`#[de(with = de_phone)]`)",
                DesensitizationType::NAMES,
            ),
        )),
    }
}

/// Builds the statement that masks a field with its desensitizer.
fn body_of(
    field: &Field,
    field_name: &Ident,
    desensitizer: &Desensitizer,
) -> Result<TokenStream2, syn::Error> {
    let inner = FieldType::extract_option_inner_type(&field.ty);
    let value_type = inner.as_ref().unwrap_or(&field.ty);

    if !FieldType::is_string(value_type) {
        return Err(syn::Error::new(
            field.ty.span(),
            "a desensitizer reads and writes text, so `de` is given for a field of \
             `String`, `Box<str>`, or `Cow<str>`, or for one of them in an `Option`",
        ));
    }

    // A value that is held behind a reference or in a slice cannot be replaced,
    // so the field is refused instead of generating code that does not compile.
    ensure_owned(&field.ty)?;
    ensure_owned(value_type)?;

    // `String` borrows as `&str` through `as_str`, and every other text type
    // through `as_ref`.
    let accessor = if is_string_type(value_type) {
        Ident::new("as_str", Span::call_site())
    } else {
        Ident::new("as_ref", Span::call_site())
    };
    let call = desensitizer.expression();

    if inner.is_some() {
        Ok(quote! {
            self.#field_name = self.#field_name
                .as_ref()
                .map(|value| #call(value.#accessor()).into())
        })
    } else {
        Ok(quote! { self.#field_name = #call(self.#field_name.#accessor()).into() })
    }
}

/// Returns an error for a type that cannot be assigned to, such as `&str`.
fn ensure_owned(ty: &syn::Type) -> Result<(), syn::Error> {
    match ty {
        syn::Type::Reference(_) | syn::Type::Slice(_) => Err(syn::Error::new(
            ty.span(),
            "a desensitizer replaces the value of a field, so `de` is not given for \
             a reference or a slice",
        )),
        _ => Ok(()),
    }
}

/// A desensitizer a `#[de]` attribute names.
enum Desensitizer {
    /// A type the framework provides, which is called as
    /// `DesensitizedUtil::<name>`.
    BuiltIn(DesensitizationType),
    /// A function of the application, which takes `&str` and answers the value
    /// of the field.
    Function(Path),
}

impl Desensitizer {
    /// Returns the expression that masks a value.
    fn expression(&self) -> TokenStream2 {
        match self {
            Self::BuiltIn(desensitization_type) => {
                let de = desensitization_type.to_ident();

                quote! { DesensitizedUtil::#de }
            }
            Self::Function(path) => quote! { #path },
        }
    }

    /// Returns whether the desensitizer is one the framework provides, which is
    /// what decides whether the util has to be imported.
    fn is_built_in(&self) -> bool {
        matches!(self, Self::BuiltIn(_))
    }
}

/// The types the framework provides for a `#[de]` attribute.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DesensitizationType {
    Email,
    Phone,
    Password,
    Name,
    IdCard,
    BankCard,
    Address,
    Ip,
}

impl DesensitizationType {
    /// The types a `de` attribute may name, in the order they are reported.
    const NAMES: &'static str = "email, phone, password, name, id_card, bank_card, address, ip";

    /// Returns the type a `de` attribute names, if it names one.
    fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "email" => Some(Self::Email),
            "phone" => Some(Self::Phone),
            "password" => Some(Self::Password),
            "name" => Some(Self::Name),
            "id_card" => Some(Self::IdCard),
            "bank_card" => Some(Self::BankCard),
            "address" => Some(Self::Address),
            "ip" => Some(Self::Ip),
            _ => None,
        }
    }

    /// Returns the name of the function of `DesensitizedUtil` that masks a value
    /// the way this type does.
    fn to_ident(self) -> Ident {
        let name = match self {
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Password => "password",
            Self::Name => "name",
            Self::IdCard => "id_card",
            Self::BankCard => "bank_card",
            Self::Address => "address",
            Self::Ip => "ip",
        };

        Ident::new(name, Span::mixed_site())
    }
}

/// Returns whether the type is `String`, which is the text type that borrows
/// through `as_str`.
fn is_string_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "String";
        }
    }

    false
}

/// Adds an error to the errors of a derive, keeping the first one.
fn push_error(errors: &mut Option<syn::Error>, error: syn::Error) {
    match errors {
        Some(existing) => existing.combine(error),
        None => *errors = Some(error),
    }
}
