use async_trait::async_trait;
use axum::body::Bytes;
use axum::http::{Response, StatusCode};
use axum::Router;
use http_body_util::Full;
use next_web_core::client::rest_client::RestClient;
use next_web_core::constants::application_constants::{
    APPLICATION_BANNER, APPLICATION_DEFAULT_PORT,
};
use next_web_core::context::application_args::ApplicationArgs;
use next_web_core::context::application_context::ApplicationContext;
use next_web_core::context::application_resources::{ApplicationResources, ResourceLoader};
use next_web_core::context::properties::{ApplicationProperties, Properties};
use next_web_core::traits::application::application_ready_event::ApplicationReadyEvent;
use next_web_core::traits::apply_router::ApplyRouter;
use next_web_core::traits::use_router::UseRouter;
use next_web_core::state::application_state::ApplicationState;
use next_web_core::AutoRegister;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

use crate::application::next_application::NextApplication;

use crate::application::permitted_groups::PERMITTED_GROUPS;
use crate::autoregister::register_single::ApplicationDefaultRegisterContainer;

use crate::banner::top_banner::{TopBanner, DEFAULT_TOP_BANNER};
use crate::event::default_application_event_multicaster::DefaultApplicationEventMulticaster;
use crate::event::default_application_event_publisher::DefaultApplicationEventPublisher;
use crate::util::local_date_time::LocalDateTime;

use next_web_core::traits::application::application_shutdown::ApplicationShutdown;

use next_web_core::traits::event::application_event_multicaster::ApplicationEventMulticaster;
use next_web_core::traits::event::application_listener::ApplicationListener;

#[cfg(feature = "scheduler")]
use crate::manager::job_scheduler_manager::JobSchedulerManager;
#[cfg(feature = "scheduler")]
use next_web_core::traits::job::application_job::ApplicationJob;

#[async_trait]
pub trait Application: Send + Sync {
    /// Initialize the middleware.
    async fn init_middleware(&mut self, properties: &ApplicationProperties);

    // Get the application router. (open api  and private api)
    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> axum::Router;

    /// Before starting the application
    #[allow(unused_variables)]
    async fn before_start(&mut self, ctx: &mut ApplicationContext) {}

    /// Register the rpc server.
    #[cfg(feature = "enable-grpc")]
    async fn register_rpc_server(&mut self, properties: &ApplicationProperties);

    /// Register the grpc client.
    #[cfg(feature = "enable-grpc")]
    async fn connect_rpc_client(&mut self, properties: &ApplicationProperties);

    /// Show the banner of the application.
    fn banner_show(application_resources: &ApplicationResources) {
        if let None = application_resources
            .load(APPLICATION_BANNER)
            .map(|content| {
                TopBanner::show(std::str::from_utf8(content.as_ref()).unwrap_or(DEFAULT_TOP_BANNER))
            })
        {
            TopBanner::show(DEFAULT_TOP_BANNER);
        }
    }

    /// Initialize the logging.
    fn init_logging(&self, application_properties: &ApplicationProperties) {
        let application_name = application_properties
            .next()
            .appliation()
            .map(|app| app.name())
            .unwrap_or_default();
        let logging = application_properties.next().logging();
        let file_appender = logging.map_or_else(
            || None,
            |logging| {
                // write log?
                if logging.write() {
                    let path = logging.log_dir().unwrap_or_else(|| "./logs");
                    let log_name = format!(
                        "{}{}.log",
                        application_name,
                        if logging.additional_date() {
                            format!("-{}", LocalDateTime::date())
                        } else {
                            String::new()
                        }
                    );
                    return Some(tracing_appender::rolling::daily(path, log_name));
                }
                None
            },
        );

        let config = tracing_subscriber::fmt::format()
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
                "%Y-%m-%d %H:%M:%S%.3f".to_string(),
            ))
            .with_level(true)
            .with_target(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_thread_names(true);

        // tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

        let logger = tracing_subscriber::fmt()
            // test
            .with_max_level(
                logging
                    .map(|log| log.level())
                    .unwrap_or(tracing::Level::INFO),
            )
            .with_ansi(false)
            .event_format(config);

        if let Some(file_appender) = file_appender {
            let (non_blocking, _worker) = tracing_appender::non_blocking(file_appender);
            logger.with_writer(non_blocking).with_test_writer().init();
        } else {
            logger.init();
        }
    }

    /// Autowire properties
    async fn autowire_properties(
        &mut self,
        ctx: &mut ApplicationContext,
        application_properties: &ApplicationProperties,
    ) {
        let properties = ctx.resolve_by_type::<Box<dyn Properties>>();
        for item in properties {
            item.register(ctx, application_properties).await.unwrap();
        }
    }

    /// Register application singleton
    async fn register_singleton(
        &self,
        ctx: &mut ApplicationContext,
        application_properties: &ApplicationProperties,
        application_args: &ApplicationArgs,
        application_resources: &ApplicationResources,
    ) {
        // Register singletion
        // properties args resources
        ctx.insert_singleton_with_name(application_properties.to_owned(), "applicationProperties");
        ctx.insert_singleton_with_name(application_args.to_owned(), "applicationArgs");
        ctx.insert_singleton_with_name(application_resources.to_owned(), "applicationResources");

        let mut container = ApplicationDefaultRegisterContainer::default();
        container.register_all(ctx, application_properties).await;

        // Resove autoRegister
        let auto_register = ctx.resolve_by_type::<Arc<dyn AutoRegister>>();
        for item in auto_register.iter() {
            item.register(ctx, application_properties).await.unwrap();
        }
    }

    /// Initialize the application infrastructure
    async fn init_infrastructure(
        &self,
        ctx: &mut ApplicationContext,
        _application_properties: &ApplicationProperties,
    ) {
        // Register job
        let producers = ctx.resolve_by_type::<Arc<dyn ApplicationJob>>();

        if let Some(job_schedluer_manager) =
            ctx.get_single_option_with_name::<JobSchedulerManager>("jobSchedulerManager")
        {
            let mut schedluer_manager = job_schedluer_manager.clone();
            for producer in producers {
                schedluer_manager.add_job(producer).await;
            }
            schedluer_manager.start();
        } else {
            warn!("Job scheduler manager not found.");
        }

        // Register application event
        let (tx, rx) = flume::unbounded();
        let mut default_event_publisher = DefaultApplicationEventPublisher::new();
        let mut multicaster = DefaultApplicationEventMulticaster::new();

        default_event_publisher.set_channel(Some(tx));
        multicaster.set_event_channel(rx);

        let listeners = ctx.resolve_by_type::<Box<dyn ApplicationListener>>();
        listeners.into_iter().for_each(|listener| {
            multicaster.add_application_listener(listener);
        });

        multicaster.run();

        let rest_client = RestClient::new();
        ctx.insert_singleton_with_name(default_event_publisher, "defaultApplicationEventPublisher");
        ctx.insert_singleton_with_name(multicaster, "defaultApplicationEventMulticaster");
        ctx.insert_singleton_with_name(rest_client, "restClient");
    }

    /// Bind tcp server.
    async fn bind_tcp_server(
        &mut self,
        mut ctx: ApplicationContext,
        application_properties: &ApplicationProperties,
        time: std::time::Instant,
    ) {
        // 1. Trigger the ApplicationReadyEvent
        for ready_event in ctx.resolve_by_type::<Box<dyn ApplicationReadyEvent>>() {
            ready_event.ready(&mut ctx).await;
        }

        // 2. Read server configuration
        let config = application_properties.next().server();
        let context_path = config.context_path().unwrap_or("");
        let server_port = config.port().unwrap_or(APPLICATION_DEFAULT_PORT);
        #[rustfmt::skip]
        let server_addr = if config.local().unwrap_or(true) { "127.0.0.1" } else { "0.0.0.0" };

        // 3. Build basic routing
        let mut app = self
            .application_router(&mut ctx)
            .await
            // Handle not found route
            .fallback(fall_back)
            // Prevent program panic caused by users not setting routes
            .route("/_20250101", axum::routing::get(|| async { "a new year!" }));

        // 4. UseRouter and ApplyRouter
        let use_routers = ctx.resolve_by_type::<Box<dyn UseRouter>>();
        app = use_routers
            .into_iter()
            // Only allowed groups can apply
            .filter(|s| PERMITTED_GROUPS.contains(&s.group().name()))
            .fold(app, |app, item| item.use_router(app, &mut ctx));

        let (mut open_routers, mut common_routers): (Vec<_>, Vec<_>) = ctx
            .resolve_by_type::<Box<dyn ApplyRouter>>()
            .into_iter()
            .partition(|item| item.open());

        // The sorting should be small and at the top
        open_routers.sort_by_key(|r| r.order());
        common_routers.sort_by_key(|r| r.order());

        app = app.merge(
            open_routers
                .into_iter()
                .chain(common_routers.into_iter())
                .map(|r| r.router(&mut ctx))
                .filter(|r| r.has_routes())
                .fold(axum::Router::new(), |acc, r| acc.merge(r)),
        );

        // 5. Obtain necessary instances
        #[cfg(not(feature = "tls-rustls"))]
        let shutdowns = ctx.resolve_by_type::<Box<dyn ApplicationShutdown>>();

        // 6. Add global middleware layer
        {
            app = app
                // Global panic handler
                .layer(CatchPanicLayer::custom(handle_panic))
                // Handler request  max timeout
                .layer(TimeoutLayer::new(std::time::Duration::from_secs(5)))
                // Cors
                .layer(
                    CorsLayer::new()
                        .allow_origin(tower_http::cors::Any)
                        .allow_methods(tower_http::cors::Any)
                        .allow_headers(tower_http::cors::Any)
                        .max_age(std::time::Duration::from_secs(60) * 10),
                );

            // Add prometheus layer
            #[cfg(feature = "enable-prometheus")]
            #[rustfmt::skip]
            {
                let (prometheus_layer, metric_handle) = axum_prometheus::PrometheusMetricLayer::pair();
                app = app.route("/metrics", axum::routing::get(|| async move { metric_handle.render() })).layer(prometheus_layer);
            }

            // Add HTTP configuration related layers
            if let Some(http) = config.http() {
                // Request
                if let Some(req) = http.request() {
                    {
                        if req.trace() {
                            app = app.route_layer(TraceLayer::new_for_http());
                        }
                        let limit = req.max_request_size().unwrap_or_default();
                        if limit > 10 {
                            app = app.route_layer(RequestBodyLimitLayer::new(limit));
                        }
                    }
                }

                // Response
                // TODO: response middleware
            }

            // Add
        }

        // 7. Add State [Context]
        app = app.route_layer(axum::Extension(ApplicationState::from_context(ctx)));

        // 8. Nest context path
        let app = match context_path.is_empty() {
            true => app,
            _ => {
                let router = Router::new();

                #[cfg(feature = "trace-log")]
                info!("Nest context path: {}", context_path);

                router.nest(context_path, app)
            }
        };

        #[rustfmt::skip]
        println!("\nApplication Listening  on:  {}", format!("{}:{}", server_addr, server_port));

        println!("Application Running    on:  {}", LocalDateTime::now());
        println!("Application StartTime  on:  {:?}", time.elapsed());
        println!("Application CurrentPid on:  {:?}\n", std::process::id());

        // 9. Start server
        let socket_addr: SocketAddr = format!("{}:{}", server_addr, server_port).parse().unwrap();

        // Turn off signal monitoring
        #[cfg(not(feature = "tls-rustls"))]
        let shutdown_signal = async move {
            let ctrl_c = async {
                tokio::signal::ctrl_c()
                    .await
                    .expect("failed to install Ctrl+C handler");
            };

            #[cfg(unix)]
            let terminate = async {
                tokio::signal::unix::SignalKind::terminate()
                    .then(|signal| signal.recv())
                    .await;
            };

            #[cfg(not(unix))]
            let terminate = std::future::pending::<()>();

            tokio::select! {
                _ = ctrl_c => info!("Received Ctrl+C. Shutting down..."),
                _ = terminate => info!("Received terminate signal. Shutting down..."),
            }

            // 执行所有 shutdown hooks
            for mut service in shutdowns {
                service.shutdown().await;
            }
        };

        // configure certificate and private key used by https
        #[cfg(feature = "tls-rustls")]
        {
            use axum_server::tls_rustls::RustlsConfig;

            let certs_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("self_signed_certs");
            let tls_config =
                RustlsConfig::from_pem_file(certs_dir.join("cert.pem"), certs_dir.join("key.pem"))
                    .await
                    .unwrap();

            let mut server = axum_server::bind_rustls(socket_addr, tls_config);
            // IMPORTANT: This is required to advertise our support for HTTP/2 websockets to the client.
            // If you use axum::serve, it is enabled by default.
            server.http_builder().http2().enable_connect_protocol();
            server.serve(app.into_make_service()).await.unwrap();
        }

        #[cfg(not(feature = "tls-rustls"))]
        {
            let listener = tokio::net::TcpListener::bind(&socket_addr).await.unwrap();

            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown_signal)
            .await
            .unwrap();
        }
    }

    /// Run the application.
    async fn run()
    where
        Self: Application + Default,
    {
        // Record application start time
        let start_time = std::time::Instant::now();

        // Get a base application instance
        let mut next_application: NextApplication<Self> = NextApplication::new();
        let properties = next_application.application_properties().clone();
        let args = next_application.application_args().clone();
        let resources = next_application.application_resources().clone();

        // Banner show
        Self::banner_show(&resources);

        let application = next_application.application();

        println!("========================================================================\n");

        application.init_logging(&properties);
        println!(
            "Init logging success!\nCurrent Time: {}\n",
            LocalDateTime::now()
        );

        let mut ctx = ApplicationContext::options()
            .allow_override(
                properties
                    .next()
                    .appliation()
                    .map(|s| s.context().allow_override())
                    .unwrap_or(false),
            )
            .auto_register();
        println!(
            "Init application context success!\nCurrent Time: {}\n",
            LocalDateTime::now()
        );

        // Autowire properties
        application.autowire_properties(&mut ctx, &properties).await;
        println!(
            "Autowire properties success!\nCurrent Time: {}\n",
            LocalDateTime::now()
        );

        // Register singleton
        application
            .register_singleton(&mut ctx, &properties, &args, &resources)
            .await;
        println!(
            "Register singleton success!\nCurrent Time: {}\n",
            LocalDateTime::now()
        );

        // Init infrastructure
        application.init_infrastructure(&mut ctx, &properties).await;
        println!(
            "Init infrastructure success!\nCurrent Time: {}\n",
            LocalDateTime::now()
        );

        // Init middleware
        application.init_middleware(&properties).await;
        println!(
            "Init middleware success!\nCurrent Time: {}\n",
            LocalDateTime::now()
        );

        #[cfg(feature = "enable-grpc")]
        {
            application.register_rpc_server(&properties).await;
            println!(
                "Register grpc server success!\nCurrent Time: {}\n",
                LocalDateTime::now()
            );

            application.connect_rpc_client(&properties).await;
            println!(
                "Connect grpc client success!\nCurrent Time: {}\n",
                LocalDateTime::now()
            );
        }

        application.before_start(&mut ctx).await;

        println!("========================================================================");

        application
            .bind_tcp_server(ctx, &properties, start_time)
            .await;
    }
}

fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> Response<Full<Bytes>> {
    error!("Application handle panic, case: {:?}", err);
    let err_str = format!(
        "internal server error, case: {:?},\ntimestamp: {}",
        err,
        LocalDateTime::timestamp()
    );
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(err_str.into())
        .unwrap()
}

/// no route match handler
async fn fall_back() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "not macth route")
}
