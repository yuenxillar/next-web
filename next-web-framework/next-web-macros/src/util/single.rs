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