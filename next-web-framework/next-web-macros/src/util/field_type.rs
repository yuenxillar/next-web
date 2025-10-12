use quote::ToTokens;
use syn::{GenericArgument, PathArguments};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Option,
    Result,
}

impl FieldType {
    pub fn from_type(ty: &syn::Type) -> FieldType {
        match ty {
            syn::Type::Array(_) => FieldType::Array,
            syn::Type::Path(type_path) => {
                if FieldType::is_result(ty) {
                    return FieldType::Result;
                }

                if FieldType::is_option(ty) {
                    return FieldType::Option;
                }

                if FieldType::is_string(&ty) {
                    return FieldType::String;
                }

                if type_path.path.segments.len() == 1 {
                    if FieldType::is_number(&ty) {
                        return FieldType::Number;
                    }

                    if FieldType::is_boolean(&ty) {
                        return FieldType::Boolean;
                    }
                }
                FieldType::Object
            }
            syn::Type::Reference(_) => {
                if FieldType::is_string(&ty) {
                    return FieldType::String;
                }
                FieldType::Object
            }
            _ => FieldType::Object,
        }
    }

    pub fn is_number(ty: &syn::Type) -> bool {
        if let syn::Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return match segment.ident.to_string().as_str() {
                    "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32"
                    | "i64" | "i128" | "isize" | "f32" | "f64" => true,
                    _ => false,
                };
            }
        }
        false
    }

    pub fn is_string(ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(type_path) => {
                if let Some(segment) = type_path.path.segments.last() {
                    return match segment.ident.to_string().as_str() {
                        "String" => true,
                        _ => ty.to_token_stream().to_string().contains("str"),
                    };
                }
                false
            }
            syn::Type::Reference(_) => ty.to_token_stream().to_string().contains("str"),
            _ => false,
        }
    }

    pub fn is_boolean(ty: &syn::Type) -> bool {
        if let syn::Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return match segment.ident.to_string().as_str() {
                    "bool" => true,
                    _ => false,
                };
            }
        }
        false
    }

    /// Check if it is an Option type
    pub fn is_option(ty: &syn::Type) -> bool {
        if let syn::Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Option";
            }
        }
        false
    }

    pub fn is_result(ty: &syn::Type) -> bool {
        if let syn::Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Result";
            }
        }
        false
    }

    /// Extract the inner type of Option type
    pub fn extract_option_inner_type(ty: &syn::Type) -> Option<syn::Type> {
        // 检查是否为 Option 类型
        if let syn::Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Option" {
                    // 提取泛型参数
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                            return Some(inner_type.clone());
                        }
                    }
                }
            }
        }

        None
    }
}
