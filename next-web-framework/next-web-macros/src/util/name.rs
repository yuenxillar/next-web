pub fn field_name_to_singleton_name(name: &str) -> String {
    if name.is_empty() {
        return name.to_string();
    }

    let mut chars = name.chars().peekable();
    let mut name = String::with_capacity(name.len());
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

pub fn singleton_name(name: &str) -> String {
    if name.is_empty() {
        return name.to_string();
    }

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
