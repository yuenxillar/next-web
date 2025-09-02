use syn::{GenericArgument, PathArguments};

pub(crate) fn singleton_name(name: &str) -> String {
    let name = name.trim().replacen("\"", "", 2);
    let mut chars = name.chars();
    match chars.next() {
        Some(first_char) => {
            let mut singleton_name = String::with_capacity(name.len());
            singleton_name.extend(first_char.to_lowercase());
            singleton_name.push_str(chars.as_str());
            singleton_name
        }
        None => name.to_string(), // Fallback for an unlikely empty string case.
    }
}

pub(crate) fn field_name_to_singleton_name(field_name: &str) -> String {
    if field_name.is_empty() {
        return field_name.to_string();
    }

    let mut chars = field_name.chars().peekable();
    let mut name = String::with_capacity(field_name.len());
    let mut capitalize_next = false;

    while let Some(c) = chars.next() {
        if c == '_' {
            // 遇到下划线，标记下一个字符需要大写
            capitalize_next = true;
        } else if capitalize_next {
            // 当前字符需要大写
            name.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            // 普通字符，直接添加（第一个字符保持小写）
            name.push(c);
        }
    }

    name
}

pub(crate) fn is_option(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

pub(crate) fn extract_option_inner_type(ty: &syn::Type) -> Option<syn::Type> {
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
