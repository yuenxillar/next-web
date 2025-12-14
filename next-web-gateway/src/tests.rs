#[cfg(test)]
mod gateway_tests {

    use crate::{
        application::gateway_application::GatewayApplication,
        properties::gateway_properties::GatewayApplicationProperties,
    };
    use std::path::{Path, PathBuf};

    #[test]
    fn test_gateway() {
        let config_file = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("application.yaml")
            .to_str()
            .unwrap()
            .to_string();
        if let Ok(config) = std::fs::read_to_string(Path::new(&config_file)) {
            if let Ok(file) = serde_yaml::from_str::<GatewayApplicationProperties>(&config) {
                println!("config file: {:?}", file);
            }
        }
    }

    #[test]
    fn test_matchit1() {
        let host = "www.example.com";
        let rule = "**.example.com";
        let host_parts: Vec<&str> = host.split('.').collect();
        let rule_parts: Vec<&str> = rule.split('.').collect();

        // 检查 host 地址和规则的段数是否一致
        if host_parts.len() != rule_parts.len() {
            return;
        }

        // 逐段匹配
        for (host_part, rule_part) in host_parts.iter().zip(rule_parts.iter()) {
            if *rule_part != "**" && *host_part != *rule_part {
                return;
            }
        }
        println!("matchit1");
    }
}
