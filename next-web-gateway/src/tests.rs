#[cfg(test)]
mod gateway_tests {

    use crate::{
        application::gateway_application::GatewayApplication,
        properties::gateway_properties::GatewayApplicationProperties,
    };
    use std::path::PathBuf;

    #[test]
    fn test_gateway() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let config_file = std::env::var(GatewayApplicationProperties::CONFIG_ENV_VAR)
            .unwrap_or_else(|_| PathBuf::from("application.yaml").display().to_string());
        let file = runtime
            .block_on(GatewayApplicationProperties::load_from_path(config_file))
            .unwrap();
        tracing::info!("config file: {:?}", file);
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
        tracing::info!("matchit1");
    }
}
