use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Data, Field, Ident};

use crate::util::field_type::FieldType;

pub(crate) fn impl_macro_desensitized(input: &syn::DeriveInput) -> TokenStream {
    let name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => panic!("error: #[derive(Desensitized)] is only supported for structs"),
    };

    // Generate desensitization code for each field
    let desensitize_fields = fields
        .iter()
        .filter(|val| val.ident.is_some() && val.attrs.len() > 0 && !val.ident.as_ref().map(|s| s.to_string().starts_with("_")).unwrap_or(false))
        .filter(|val| FieldType::is_string(&val.ty))
        .map(|field| {
            // Check if the field type is valid
            match  &field.ty {
                syn::Type::Reference(_) | syn::Type::Slice(_) => panic!("Reference type and slice type are not supported"),
                _ => {}
            }

            let field_name = field.ident.as_ref().unwrap();
            let desens_type = find_desensitization_type(field);

            match desens_type {
                Some(desens_type) => {
                    let de = desens_type.to_ident();
                    // println!("de = {}", de.to_string());
                    match desens_type {
                        DesensitizationType::Generic(_start, _end) => todo!(),
                        _ => {
                            if FieldType::is_option(&field.ty) {
                                let is_string = is_string(&field.ty);
                                if is_string {
                                    quote! { self.#field_name = self.#field_name.as_ref().map(| val | DesensitizedUtil::#de(val.as_str())) }
                                }else {
                                    quote! { self.#field_name = self.#field_name.as_ref().map(| val | DesensitizedUtil::#de(val.as_ref()).into()) }
                                }
                            }else {
                                let is_string = is_string(&field.ty);

                                if is_string {
                                    quote! { self.#field_name = DesensitizedUtil::#de(self.#field_name.as_str()) }
                                }else {
                                    // Box<str>  Cow<'static, str> Others ???
                                    quote! { self.#field_name = DesensitizedUtil::#de(self.#field_name.as_ref()).into() }
                                }
                            }
                        }
                    }
                }
                None => quote! {},
            }
        }).filter(|val| !val.is_empty()).collect::<Vec<_>>();

    let expanded = quote! {
        impl ::next_web_core::interface::desensitized::Desensitized for #name {
            fn desensitize(&mut self) {
                use ::next_web_dev::util::desensitized::DesensitizedUtil;

                #(#desensitize_fields;)*
            }
        }
    };

    // println!("expanded: {}", expanded.to_string());

    TokenStream::from(expanded)
}

/// 查找字段的脱敏类型
/// 
/// Search for the desensitization type of a field
fn find_desensitization_type(field: &Field) -> Option<DesensitizationType> {
    for attr in &field.attrs {
        if attr.path().is_ident("de") {
            if let Ok(ident) = attr.parse_args::<Ident>() {
                return DesensitizationType::from_str(ident.to_string().as_str());
            }
        }
    }
    None
}

#[derive(PartialEq, Eq)]
enum DesensitizationType {
    Email,
    Phone,
    Password,
    Name,
    IdCard,
    BankCard,
    Address,
    Ip,
    Generic(usize, usize),
}

impl DesensitizationType {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "email" => Some(DesensitizationType::Email),
            "phone" => Some(DesensitizationType::Phone),
            "password" => Some(DesensitizationType::Password),
            "name" => Some(DesensitizationType::Name),
            "id_card" => Some(DesensitizationType::IdCard),
            "bank_card" => Some(DesensitizationType::BankCard),
            "address" => Some(DesensitizationType::Address),
            "ip" => Some(DesensitizationType::Ip),
            // "generic" => {
            //     Some(DesensitizationType::Generic( 0, 0))
            // }
            _ => panic!("\nUnsupported desensitization type: {}\nThe supported parameters are as follows: \nemail, phone, name, id_card, bank_card, address, ip", s),
        }
    }

    fn to_ident(&self) -> Ident {
        let s = match self {
            DesensitizationType::Email => "email",
            DesensitizationType::Phone => "phone",
            DesensitizationType::Password => "password",
            DesensitizationType::Name => "name",
            DesensitizationType::IdCard => "id_card",
            DesensitizationType::BankCard => "bank_card",
            DesensitizationType::Address => "address",
            DesensitizationType::Ip => "ip",
            DesensitizationType::Generic(_, _) => "generic",
        };
        Ident::new(s, Span::mixed_site())
    }
}

// fn get_partial_params(field: &Field) -> (usize, usize) {
//     for attr in &field.attrs {
//         if attr.path().is_ident("de") {
//             let meta = attr.parse_meta().expect("Failed to parse attribute meta");
//             if let syn::Meta::List(meta_list) = meta {
//                 if let Some(syn::NestedMeta::Meta(syn::Meta::NameValue(name_value))) = meta_list.nested.first() {
//                     if name_value.path.is_ident("partial") {
//                         if let syn::Lit::Str(lit_str) = &name_value.lit {
//                             let value = lit_str.value();
//                             let parts: Vec<&str> = value.split(',').collect();
//                             if parts.len() == 2 {
//                                 let prefix = parts[0].trim().parse().unwrap_or(3);
//                                 let suffix = parts[1].trim().parse().unwrap_or(4);
//                                 return (prefix, suffix);
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     }
//     (3, 4) // 默认值
// }


fn is_string(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 {
            return type_path.path.segments.first()
            .map(|val| val.ident == "String")
            .unwrap_or(false);
        }
    }
    false
}