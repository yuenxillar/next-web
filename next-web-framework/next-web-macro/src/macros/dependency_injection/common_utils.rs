pub(super) struct CommonUtils;

impl CommonUtils {
    // 将 test_name 转换为 testName
    pub fn generate_bean_name(name: &str) -> String {
        if name.is_empty() {
            return String::new();
        }

        if !name.contains("_") {
            return name.to_string();
        }

        name.split('_')
            .enumerate()
            .map(|(i, part)| {
                if i == 0 {
                    part.to_lowercase()
                } else {
                    let mut c = part.chars();
                    match c.next() {
                        None => String::new(),
                        Some(first_char) => {
                            first_char.to_uppercase().collect::<String>() + c.as_str()
                        }
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_bean_name() {
        println!("{:?}", CommonUtils::generate_bean_name("test_name_cici"));
    }
}