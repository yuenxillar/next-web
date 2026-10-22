//! The `Gateway` facade: load a configuration, check it, or run the server.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::Parser;
use pingora::proxy::http_proxy_service;
use pingora::server::Server;
use pingora::services::ServiceWithDependents;
use tracing_subscriber::EnvFilter;

use crate::admin::AdminService;
use crate::config::{ConfigErrors, GatewayConfig, LogFormat};
use crate::proxy::GatewayProxy;

/// Command line contract of the binary. Every flag has an environment variable.
#[derive(Debug, Clone, Parser)]
#[command(name = "next-web-gateway", version, about = "A Spring Cloud Gateway equivalent")]
pub struct Cli {
    /// Path to the configuration file.
    #[arg(short, long, env = "NEXT_WEB_GATEWAY_CONFIG", default_value = "application.yaml")]
    pub config: PathBuf,

    /// Override `next.gateway.server.bind`.
    #[arg(long, env = "NEXT_WEB_GATEWAY_BIND")]
    pub bind: Option<String>,

    /// Override `next.gateway.server.admin.bind`.
    #[arg(long, env = "NEXT_WEB_GATEWAY_ADMIN_BIND")]
    pub admin_bind: Option<String>,

    /// Override the log level (`RUST_LOG` wins when set).
    #[arg(long, env = "NEXT_WEB_GATEWAY_LOG_LEVEL")]
    pub log_level: Option<String>,

    /// Log format: `text` or `json`.
    #[arg(long, env = "NEXT_WEB_GATEWAY_LOG_FORMAT")]
    pub log_format: Option<String>,

    /// Validate the configuration and exit: 0 valid, 2 invalid.
    #[arg(long)]
    pub check: bool,

    /// Run in the background and manage the pid file.
    #[arg(short, long)]
    pub daemon: bool,

    /// Upgrade a running instance in place.
    #[arg(short, long)]
    pub upgrade: bool,

    /// Validate the configuration, then exit (pingora flag).
    #[arg(short = 't', long = "test")]
    pub test: bool,
}

/// Exit code for an invalid configuration.
pub const EXIT_CONFIG: i32 = 2;
/// Exit code for a runtime failure.
pub const EXIT_RUNTIME: i32 = 1;

/// The gateway: configuration plus the runtime that serves it.
pub struct Gateway {
    config: GatewayConfig,
    config_path: Option<PathBuf>,
    bind: Option<String>,
    admin_bind: Option<String>,
    daemon: bool,
    upgrade: bool,
    test: bool,
}

impl Gateway {
    /// Load the configuration from a file.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigErrors> {
        let path = path.as_ref();
        let config = GatewayConfig::from_path(path)?;

        Ok(Self {
            config,
            config_path: Some(path.to_path_buf()),
            bind: None,
            admin_bind: None,
            daemon: false,
            upgrade: false,
            test: false,
        })
    }

    /// Parse the configuration from a string.
    pub fn parse_str(text: &str) -> Result<Self, ConfigErrors> {
        use std::str::FromStr;

        Ok(Self {
            config: GatewayConfig::from_str(text)?,
            config_path: None,
            bind: None,
            admin_bind: None,
            daemon: false,
            upgrade: false,
            test: false,
        })
    }

    /// The parsed configuration.
    pub fn config(&self) -> &GatewayConfig {
        &self.config
    }

    /// Apply the command line overrides.
    pub fn with_overrides(mut self, cli: &Cli) -> Self {
        self.bind = cli.bind.clone();
        self.admin_bind = cli.admin_bind.clone();
        self.daemon = cli.daemon;
        self.upgrade = cli.upgrade;
        self.test = cli.test;
        self
    }

    /// Build the proxy, which also validates every route and filter.
    pub fn build(self) -> Result<(GatewayProxy, GatewayConfig, Option<PathBuf>), ConfigErrors> {
        let proxy = GatewayProxy::new(self.config.clone())?;
        Ok((proxy, self.config, self.config_path))
    }

    /// Validate the configuration and return the route ids it defines.
    pub fn check(&self) -> Result<Vec<String>, ConfigErrors> {
        self.validate_bindings()?;
        let proxy = GatewayProxy::new(self.config.clone())?;
        Ok(proxy.routes().ordered_route_ids().to_vec())
    }

    /// Check the effective listen addresses, including the CLI overrides.
    fn validate_bindings(&self) -> Result<(), ConfigErrors> {
        let mut errors = Vec::new();

        let bind = self.bind();
        if !crate::config::is_valid_bind(&bind) {
            errors.push(crate::config::ConfigError::new(
                "bind",
                format!("`{bind}` is not a `host:port` listen address"),
            ));
        }

        if self.config.server.admin.enabled {
            let admin_bind = self.admin_bind();
            if !crate::config::is_valid_bind(&admin_bind) {
                errors.push(crate::config::ConfigError::new(
                    "admin.bind",
                    format!("`{admin_bind}` is not a `host:port` listen address"),
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ConfigErrors::new(errors))
        }
    }

    /// The configured bind address.
    pub fn bind(&self) -> String {
        self.bind
            .clone()
            .unwrap_or_else(|| self.config.server.bind.clone())
    }

    /// The configured admin bind address.
    pub fn admin_bind(&self) -> String {
        self.admin_bind
            .clone()
            .unwrap_or_else(|| self.config.server.admin.bind.clone())
    }

    /// Run the gateway until it is shut down.
    pub fn run(self) -> ! {
        if let Err(error) = self.validate_bindings() {
            eprintln!("{error}");
            std::process::exit(EXIT_CONFIG);
        }

        let bind = self.bind();
        let admin_bind = self.admin_bind();

        let proxy = match GatewayProxy::new(self.config.clone()) {
            Ok(proxy) => proxy,
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(EXIT_CONFIG);
            }
        };

        let routes = proxy.shared_routes();
        let metrics = proxy.metrics().clone();
        let ready = proxy.readiness();
        let admin = AdminService::new(Arc::new(self.config.clone()), routes, metrics, ready);

        // Apply the listener settings that pingora can honour instead of
        // silently running on its defaults.
        let mut conf = pingora::server::configuration::ServerConf::default();
        if self.config.server.threads > 0 {
            conf.threads = self.config.server.threads;
        }
        let shutdown = self.config.server.graceful_shutdown.as_secs().max(1);
        conf.grace_period_seconds = Some(shutdown);
        conf.graceful_shutdown_timeout_seconds = Some(shutdown);

        // `-d`/`-u`/`-t` are pingora flags: forwarding them means `--daemon`
        // really backgrounds the process and `--test` really validates the
        // startup instead of being parsed and dropped.
        let opt = pingora::server::configuration::Opt {
            daemon: self.daemon,
            upgrade: self.upgrade,
            test: self.test,
            ..Default::default()
        };

        let mut server = Server::new_with_opt_and_conf(Some(opt), conf);

        server.bootstrap();

        let mut services: Vec<Box<dyn ServiceWithDependents>> = Vec::new();

        let mut proxy_service = http_proxy_service(&server.configuration, proxy);
        proxy_service.add_tcp(&bind);
        services.push(Box::new(proxy_service));

        if self.config.server.admin.enabled {
            let mut admin_service = http_proxy_service(&server.configuration, admin);
            admin_service.add_tcp(&admin_bind);
            services.push(Box::new(admin_service));
        }

        tracing::info!(
            bind = %bind,
            admin = %admin_bind,
            routes = self.config.routes.len(),
            "gateway started"
        );

        server.add_services(services);
        server.run_forever();
    }
}

/// Initialise tracing from the CLI and the configuration.
pub fn init_logging(cli: &Cli, config: Option<&GatewayConfig>) {
    let level = cli
        .log_level
        .clone()
        .or_else(|| config.map(|config| config.logging.level.clone()))
        .unwrap_or_else(|| "info".to_owned());

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    let format = cli
        .log_format
        .as_deref()
        .map(|format| format.eq_ignore_ascii_case("json"))
        .unwrap_or_else(|| {
            config
                .map(|config| config.logging.format == LogFormat::Json)
                .unwrap_or(false)
        });

    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    if format {
        builder.json().init();
    } else {
        builder.init();
    }
}

/// Parse the CLI and run the binary, mapping failures to exit codes.
pub fn main() {
    let cli = Cli::parse();

    let gateway = match Gateway::from_path(&cli.config) {
        Ok(gateway) => gateway.with_overrides(&cli),
        Err(error) => {
            // Logging may not be initialised yet, so report on stderr.
            eprintln!("{error}");
            std::process::exit(EXIT_CONFIG);
        }
    };

    let document = gateway.config().clone();
    init_logging(&cli, Some(&document));

    if cli.check {
        match gateway.check() {
            Ok(route_ids) => {
                println!("configuration is valid: {} route(s)", route_ids.len());
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(EXIT_CONFIG);
            }
        }
    }

    gateway.run()
}

#[cfg(test)]
mod tests {
    use super::{Cli, Gateway, EXIT_CONFIG};
    use clap::Parser;

    const VALID: &str = "\
next:
  gateway:
    routes:
      - id: users
        uri: http://backend:8080
        predicates: ['Path=/api/users/**']
";

    #[test]
    fn cli_defaults_to_application_yaml() {
        let cli = Cli::try_parse_from(["next-web-gateway"]).unwrap();
        assert_eq!(cli.config.to_string_lossy(), "application.yaml");
        assert!(!cli.check);
    }

    #[test]
    fn cli_reads_the_check_flag_and_config_path() {
        let cli =
            Cli::try_parse_from(["next-web-gateway", "--check", "--config", "gateway.yaml"]).unwrap();
        assert!(cli.check);
        assert_eq!(cli.config.to_string_lossy(), "gateway.yaml");
    }

    #[test]
    fn check_returns_the_route_ids() {
        let gateway = Gateway::parse_str(VALID).unwrap();
        assert_eq!(gateway.check().unwrap(), vec!["users".to_owned()]);
    }

    #[test]
    fn check_rejects_an_unknown_filter() {
        let yaml = "\
next:
  gateway:
    routes:
      - id: users
        uri: http://backend:8080
        filters: ['StrpPrefix=1']
";
        let gateway = Gateway::parse_str(yaml).unwrap();
        let error = gateway.check().unwrap_err().to_string();
        assert!(error.contains("StripPrefix"), "{error}");
    }

    #[test]
    fn exit_code_for_an_invalid_configuration_is_two() {
        assert_eq!(EXIT_CONFIG, 2);
    }

    #[test]
    fn overrides_take_precedence_over_the_file() {
        let cli = Cli::try_parse_from(["next-web-gateway", "--bind", "0.0.0.0:9000"]).unwrap();
        let gateway = Gateway::parse_str(VALID).unwrap().with_overrides(&cli);

        assert_eq!(gateway.bind(), "0.0.0.0:9000");
        assert_eq!(gateway.admin_bind(), "127.0.0.1:9001");
    }

    #[test]
    fn a_bad_bind_override_is_a_configuration_error() {
        let cli = Cli::try_parse_from(["next-web-gateway", "--bind", "nonsense"]).unwrap();
        let gateway = Gateway::parse_str(VALID).unwrap().with_overrides(&cli);

        let error = gateway.check().unwrap_err().to_string();
        assert!(error.contains("bind"), "{error}");
    }

    #[test]
    fn a_bad_admin_bind_override_is_a_configuration_error() {
        let cli = Cli::try_parse_from(["next-web-gateway", "--admin-bind", "9001"]).unwrap();
        let gateway = Gateway::parse_str(VALID).unwrap().with_overrides(&cli);

        assert!(gateway.check().is_err());
    }
}
