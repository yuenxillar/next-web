use crate::util::key_value::KeyValue;

pub struct StrUtil;

impl StrUtil {
    // Header=X-Request-Id, ch.p
    pub fn parse_kv_one_and_option(str: &str) -> KeyValue<Option<String>> {
        str.trim_end()
            .split_once(',')
            .map(|(n, v)| KeyValue {
                k: n.into(),
                v: Some(v.to_string()),
            })
            .unwrap_or(KeyValue {
                k: str.trim_end().into(),
                v: None,
            })
    }

    // **.baidu.**
    pub fn parse_wildcard_pattern(pattern: &str) -> Option<usize> {
        let parts: Vec<&str> = pattern.split('.').collect();

        // 记录 ** 的位置
        let mut wildcard_positions = Vec::new();
        for (index, part) in parts.iter().enumerate() {
            if *part == "**" {
                wildcard_positions.push(index);
            }
        }
        if wildcard_positions.len() == 1 {
            return Some(wildcard_positions[0]);
        }
        return Some(wildcard_positions.iter().sum::<usize>() + 1);
    }

    pub fn host_match(host: &str, rule: &str) -> bool {
        if host.eq(rule) {
            return true;
        }

        let host_parts: Vec<&str> = host.split('.').collect();
        let rule_parts: Vec<&str> = rule.split('.').collect();

        // 检查 host 地址和规则的段数是否一致
        if host_parts.len() != rule_parts.len() {
            return false;
        }

        // 逐段匹配
        for (host_part, rule_part) in host_parts.iter().zip(rule_parts.iter()) {
            if *rule_part != "**" && *host_part != *rule_part {
                return false;
            }
        }

        true
    }
}
