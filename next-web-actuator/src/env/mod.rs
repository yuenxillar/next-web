use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Snapshot of the current process environment focused on Rust-related data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvReport {
    pub summary: EnvSummary,
    pub property_sources: Vec<EnvPropertySource>,
}

impl EnvReport {
    /// Collects a report from the current process environment.
    pub fn collect() -> Self {
        Self::from_iter(std::env::vars())
    }

    /// Collects a report from an arbitrary environment iterator.
    pub fn from_iter<I, K, V>(vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let vars: Vec<(String, String)> = vars
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();

        let rust_related_count = vars
            .iter()
            .filter(|(key, _)| is_rust_related_env(key))
            .count();

        Self {
            summary: EnvSummary {
                total_vars: vars.len(),
                rust_related_vars: rust_related_count,
                platform: PlatformInfo::current(),
            },
            property_sources: vec![
                EnvPropertySource {
                    name: "compile-time".to_string(),
                    properties: compile_time_properties(),
                },
                EnvPropertySource {
                    name: "rust-env".to_string(),
                    properties: collect_filtered_source(&vars, |key| key.starts_with("RUST_")),
                },
                EnvPropertySource {
                    name: "cargo-env".to_string(),
                    properties: collect_filtered_source(&vars, |key| key.starts_with("CARGO_")),
                },
                EnvPropertySource {
                    name: "rustup-env".to_string(),
                    properties: collect_filtered_source(&vars, |key| key.starts_with("RUSTUP_")),
                },
                EnvPropertySource {
                    name: "tooling-env".to_string(),
                    properties: collect_filtered_source(&vars, |key| {
                        matches!(
                            key,
                            "RUST_LOG"
                                | "RUST_BACKTRACE"
                                | "RUSTFLAGS"
                                | "RUSTDOCFLAGS"
                                | "CARGO_BUILD_TARGET"
                                | "CARGO_HOME"
                        )
                    }),
                },
            ],
        }
    }

    /// Returns only the non-empty property sources.
    pub fn non_empty_sources(&self) -> Vec<&EnvPropertySource> {
        self.property_sources
            .iter()
            .filter(|source| !source.properties.is_empty())
            .collect()
    }
}

/// Basic summary for the environment snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvSummary {
    pub total_vars: usize,
    pub rust_related_vars: usize,
    pub platform: PlatformInfo,
}

/// One logical source of environment properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvPropertySource {
    pub name: String,
    pub properties: BTreeMap<String, EnvValue>,
}

/// A single environment property value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvValue {
    pub value: String,
    pub masked: bool,
    pub origin: EnvValueOrigin,
}

/// Origin metadata for one environment value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvValueOrigin {
    RuntimeEnv,
    CompileTime,
}

/// Platform details that help interpret env values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    pub family: String,
    pub arch: String,
}

impl PlatformInfo {
    fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            family: std::env::consts::FAMILY.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        }
    }
}

fn collect_filtered_source<F>(vars: &[(String, String)], predicate: F) -> BTreeMap<String, EnvValue>
where
    F: Fn(&str) -> bool,
{
    vars.iter()
        .filter(|(key, _)| predicate(key))
        .map(|(key, value)| {
            (
                key.clone(),
                EnvValue {
                    value: redact_env_value(key, value),
                    masked: should_mask_env_value(key),
                    origin: EnvValueOrigin::RuntimeEnv,
                },
            )
        })
        .collect()
}

fn compile_time_properties() -> BTreeMap<String, EnvValue> {
    let mut properties = BTreeMap::new();

    insert_compile_time(
        &mut properties,
        "CARGO_PKG_NAME",
        option_env!("CARGO_PKG_NAME"),
    );
    insert_compile_time(
        &mut properties,
        "CARGO_PKG_VERSION",
        option_env!("CARGO_PKG_VERSION"),
    );
    insert_compile_time(
        &mut properties,
        "CARGO_PKG_DESCRIPTION",
        option_env!("CARGO_PKG_DESCRIPTION"),
    );
    insert_compile_time(
        &mut properties,
        "CARGO_PKG_AUTHORS",
        option_env!("CARGO_PKG_AUTHORS"),
    );
    insert_compile_time(
        &mut properties,
        "CARGO_PKG_REPOSITORY",
        option_env!("CARGO_PKG_REPOSITORY"),
    );
    insert_compile_time(
        &mut properties,
        "CARGO_PKG_LICENSE",
        option_env!("CARGO_PKG_LICENSE"),
    );
    insert_compile_time(
        &mut properties,
        "RUSTUP_TOOLCHAIN",
        option_env!("RUSTUP_TOOLCHAIN"),
    );
    insert_compile_time(&mut properties, "TARGET", option_env!("TARGET"));
    insert_compile_time(&mut properties, "HOST", option_env!("HOST"));
    insert_compile_time(&mut properties, "PROFILE", option_env!("PROFILE"));
    insert_compile_time(&mut properties, "OPT_LEVEL", option_env!("OPT_LEVEL"));
    insert_compile_time(&mut properties, "DEBUG", option_env!("DEBUG"));

    properties
}

fn insert_compile_time(
    properties: &mut BTreeMap<String, EnvValue>,
    key: &str,
    value: Option<&'static str>,
) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        properties.insert(
            key.to_string(),
            EnvValue {
                value: redact_env_value(key, value),
                masked: should_mask_env_value(key),
                origin: EnvValueOrigin::CompileTime,
            },
        );
    }
}

fn is_rust_related_env(key: &str) -> bool {
    key.starts_with("RUST_") || key.starts_with("CARGO_") || key.starts_with("RUSTUP_")
}

fn should_mask_env_value(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "KEY",
        "CREDENTIAL",
        "AUTH",
        "PRIVATE",
    ]
    .iter()
    .any(|marker| upper.contains(marker))
}

fn redact_env_value(key: &str, value: &str) -> String {
    if !should_mask_env_value(key) {
        return value.to_string();
    }

    if value.is_empty() {
        return String::new();
    }

    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 4 {
        return "*".repeat(chars.len());
    }

    let prefix: String = chars.iter().take(2).collect();
    let suffix: String = chars
        .iter()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{prefix}***{suffix}")
}

#[cfg(test)]
mod tests {
    use super::{EnvReport, EnvValueOrigin, redact_env_value, should_mask_env_value};

    #[test]
    fn collects_rust_related_sources() {
        let report = EnvReport::from_iter([
            ("RUST_LOG", "debug"),
            ("CARGO_HOME", "/tmp/cargo"),
            ("RUSTUP_HOME", "/tmp/rustup"),
            ("PATH", "/usr/bin"),
        ]);

        assert_eq!(report.summary.total_vars, 4);
        assert_eq!(report.summary.rust_related_vars, 3);
        assert!(
            report
                .property_sources
                .iter()
                .any(|source| source.name == "rust-env"
                    && source.properties.contains_key("RUST_LOG"))
        );
        assert!(report.property_sources.iter().any(
            |source| source.name == "cargo-env" && source.properties.contains_key("CARGO_HOME")
        ));
        assert!(
            report
                .property_sources
                .iter()
                .any(|source| source.name == "rustup-env"
                    && source.properties.contains_key("RUSTUP_HOME"))
        );
    }

    #[test]
    fn masks_sensitive_values() {
        assert!(should_mask_env_value("CARGO_REGISTRY_TOKEN"));
        assert_eq!(
            redact_env_value("CARGO_REGISTRY_TOKEN", "abcdef123456"),
            "ab***56"
        );
        assert_eq!(redact_env_value("RUST_LOG", "debug"), "debug");
    }

    #[test]
    fn compile_time_source_is_tagged() {
        let report = EnvReport::collect();
        let compile_time = report
            .property_sources
            .iter()
            .find(|source| source.name == "compile-time")
            .unwrap();

        for value in compile_time.properties.values() {
            assert_eq!(value.origin, EnvValueOrigin::CompileTime);
        }
    }
}
