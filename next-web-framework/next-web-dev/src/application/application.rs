use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Router;

use next_web_core::async_trait;
use next_web_core::client::rest_client::RestClient;
use next_web_core::constants::application_constants::{
    APPLICATION_BANNER, APPLICATION_DEFAULT_PORT,
};
use next_web_core::context::application_args::ApplicationArgs;
use next_web_core::context::application_context::ApplicationContext;
use next_web_core::context::application_resources::{ApplicationResources, ResourceLoader};
use next_web_core::context::properties::{ApplicationProperties, Properties};
use next_web_core::state::application_state::ApplicationState;
use next_web_core::traits::application::application_ready_event::ApplicationReadyEvent;
use next_web_core::traits::apply_router::ApplyRouter;
use next_web_core::traits::error_solver::ErrorSolver;
use next_web_core::traits::properties_post_processor::PropertiesPostProcessor;
use next_web_core::traits::use_router::UseRouter;
use next_web_core::AutoRegister;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
#[allow(unused_imports)]
use tracing::{error, info};

use crate::application::next_application::NextApplication;

use crate::application::permitted_groups::PERMITTED_GROUPS;
use crate::autoregister::default_autoregister::DefaultAutoRegister;
use crate::autoregister::handler_autoregister::HttpHandlerAutoRegister;

use crate::banner::top_banner::{TopBanner, DEFAULT_TOP_BANNER};
use crate::configurer::http_method_handler_configurer::{RouteState, RouterContext};
use crate::event::default_application_event_multicaster::DefaultApplicationEventMulticaster;
use crate::event::default_application_event_publisher::DefaultApplicationEventPublisher;
use crate::util::local_date_time::LocalDateTime;

use next_web_core::traits::application::application_shutdown::ApplicationShutdown;

use next_web_core::traits::event::application_event_multicaster::ApplicationEventMulticaster;
use next_web_core::traits::event::application_listener::ApplicationListener;

#[cfg(feature = "enable-scheduling")]
use crate::autoregister::scheduler_autoregister::SchedulerAutoRegister;
#[cfg(feature = "enable-scheduling")]
use crate::manager::job_scheduler_manager::JobSchedulerManager;
#[cfg(feature = "enable-scheduling")]
#[allow(unused_imports)]
use next_web_core::traits::schedule::scheduled_task::ScheduledTask;

#[async_trait]
pub trait Application
where
    Self: Send + Sync,
    Self: 'static,
{
    /// The error solver for the application.
    ///
    /// Apply it to the `catch_panic` function
    type ErrorSolve: ErrorSolver;

    /// Initialize the middleware.
    async fn init_middleware(&self, properties: &ApplicationProperties);

    /// Before starting the application
    #[allow(unused_variables)]
    async fn on_ready(&self, ctx: &mut ApplicationContext) {}

    /// Register the rpc server.
    #[cfg(feature = "enable-grpc")]
    async fn register_rpc_server(
        &self,
        ctx: &mut ApplicationContext,
        application_properties: &ApplicationProperties,
        application_args: &ApplicationArgs,
        application_resources: &ApplicationResources,
    );

    /// Register the grpc client.
    #[cfg(feature = "enable-grpc")]
    async fn connect_rpc_client(
        &self,
        ctx: &mut ApplicationContext,
        application_properties: &ApplicationProperties,
        application_args: &ApplicationArgs,
        application_resources: &ApplicationResources,
    );

    /// Show the banner of the application.
    fn banner_show(application_resources: &ApplicationResources) {
        if let Some(content) = application_resources.load(APPLICATION_BANNER) {
            if let Ok(txt) = std::str::from_utf8(content.as_ref()) {
                TopBanner::show(txt);
                return;
            }
        };

        TopBanner::show(DEFAULT_TOP_BANNER);
    }

    /// Suitable for capturing panic in application
    fn catch_panic(err: Box<dyn std::any::Any + Send + 'static>) -> Response {
        let error = if let Some(msg) = err.downcast_ref::<String>() {
            msg.to_string()
        } else if let Some(msg) = err.downcast_ref::<&str>() {
            msg.to_string()
        } else {
            error!("Service panicked but `CatchPanic` was unable to downcast the panic info");
            String::with_capacity(0)
        };

        error!("Service panicked: {}", &error);

        let mut resp = Self::ErrorSolve::solve_error(error).into_response();

        *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;

        resp
    }

    /// No matching route handler
    async fn fallback() -> Response {
        let mut resp = Self::ErrorSolve::solve_error(String::from("Not Found")).into_response();

        *resp.status_mut() = StatusCode::NOT_FOUND;

        resp
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

        let default_format = "%Y-%m-%d %H:%M:%S%.3f";
        let config = tracing_subscriber::fmt::format()
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
                logging
                    .map(|val| {
                        val.format()
                            .map(ToString::to_string)
                            .unwrap_or(default_format.to_string())
                    })
                    .unwrap_or(default_format.to_string()),
            ))
            .with_level(true)
            .with_target(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_ansi(true)
            .with_source_location(true)
            .with_thread_ids(true)
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
        &self,
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
        // [properties] [args] [resources]
        ctx.insert_singleton_with_default_name(application_properties.to_owned());
        ctx.insert_singleton_with_default_name(application_args.to_owned());
        ctx.insert_singleton_with_default_name(application_resources.to_owned());

        // If a declarative macro is used for submission, it should not be found in the Application Context
        for default_auto_register in inventory::iter::<&dyn DefaultAutoRegister>.into_iter() {
            default_auto_register
                .register(ctx, application_properties)
                .await
                .unwrap();
        }

        // Resove autoRegister
        let auto_registers = ctx.resolve_by_type::<Arc<dyn AutoRegister>>();
        for auto_register in auto_registers.iter() {
            auto_register
                .register(ctx, application_properties)
                .await
                .unwrap();
        }
    }

    /// Initialize the application infrastructure
    async fn init_infrastructure(
        &self,
        ctx: &mut ApplicationContext,
        _application_properties: &ApplicationProperties,
    ) {
        // Register application event
        let (tx, rx) = flume::unbounded();
        let mut default_event_publisher = DefaultApplicationEventPublisher::new();
        let mut multicaster = DefaultApplicationEventMulticaster::new();

        default_event_publisher.set_channel(Some(tx));
        multicaster.set_event_channel(rx);

        let listeners = ctx.resolve_by_type::<Box<dyn ApplicationListener>>();
        for listener in listeners.into_iter() {
            multicaster.add_application_listener(listener).await;
        }

        multicaster.run();

        // Register jobs
        #[cfg(feature = "enable-scheduling")]
        {
            let mut manager = JobSchedulerManager::with_channel_size(240).await;
            for scheduler in inventory::iter::<&dyn SchedulerAutoRegister>.into_iter() {
                if let Err(error) = manager.add(scheduler.register(ctx)).await {
                    error!("JobSchedulerManager Failed to add job: {}", error);
                }
            }

            // let producers = ctx.resolve_by_type::<Arc<dyn ApplicationJob>>();
            // for producer in producers {
            //     manager.add_job(producer).await;
            // }

            manager.start().await;

            ctx.insert_singleton_with_default_name(manager);
        }

        let rest_client = RestClient::new();
        ctx.insert_singleton_with_default_name(default_event_publisher);
        ctx.insert_singleton_with_default_name(multicaster);
        ctx.insert_singleton_with_default_name(rest_client);
    }

    // Get the application router.
    #[allow(unused_variables)]
    async fn application_router(&self, ctx: &mut ApplicationContext) -> Router {
        let mut context = RouterContext::default();

        let iterator = inventory::iter::<&dyn HttpHandlerAutoRegister>
            .into_iter()
            .collect::<Vec<_>>();

        let mut router = Router::new();
        while let Some(state) = context.next() {
            let len = iterator.len();

            for index in 0..len {
                let item = &iterator[index];
                match state {
                    RouteState::End => break,
                    _ => {
                        router = item.register(router, &mut context);
                    }
                }
            }
        }
        router
    }

    /// Bind tcp server.
    async fn bind_tcp_server(
        &self,
        mut ctx: ApplicationContext,
        application_properties: &ApplicationProperties,
        time: std::time::Instant,
    ) {
        // 1. Read server configuration
        let config = application_properties.next().server();
        let context_path = config.context_path().unwrap_or("");
        let server_port = config.port().unwrap_or(APPLICATION_DEFAULT_PORT);

        let server_addr = if let Some(addr) = config.addr() {
            addr
        } else {
            if config.local().unwrap_or(true) {
                "127.0.0.1"
            } else {
                "0.0.0.0"
            }
        };

        let req_timeout = config.http().map(|http| {
            http.request()
                .map(|req| req.timeout().unwrap_or(5))
                .unwrap_or(5)
        });

        // 2. Build basic routing
        let mut app = self
            .application_router(&mut ctx)
            .await
            // Handle not found route
            .fallback(Self::fallback)
            // Prevent program panic caused by users not setting routes
            .route("/_20250101", axum::routing::get(|| async { "a new year!" }));

        // 3. UseRouter and ApplyRouter
        let use_routers = ctx.resolve_by_type::<Box<dyn UseRouter>>();
        app = use_routers
            .into_iter()
            // Only allowed groups can apply
            .filter(|s| PERMITTED_GROUPS.contains(&s.group().name()))
            .fold(app, |app, item| item.use_router(app, &mut ctx));

        let mut apply_routers: Vec<_> = ctx.resolve_by_type::<Box<dyn ApplyRouter>>();

        // The sorting should be small and at the top
        apply_routers.sort_by_key(|r| r.order());

        app = app.merge(
            apply_routers
                .into_iter()
                .map(|val| val.router(&mut ctx))
                .filter(|val| val.has_routes())
                .fold(axum::Router::new(), |acc, r| acc.merge(r)),
        );

        // 4. Obtain necessary instances
        #[cfg(not(feature = "tls-rustls"))]
        let shutdowns = ctx.resolve_by_type::<Box<dyn ApplicationShutdown>>();

        // 5. Add global middleware layer
        {
            // Add prometheus layer
            #[cfg(feature = "enable-prometheus")]
            #[rustfmt::skip]
            {
                let (prometheus_layer, metric_handle) = axum_prometheus::PrometheusMetricLayer::pair();
                app = app.route("/metrics", axum::routing::get(|| async move { metric_handle.render() })).layer(prometheus_layer);
            }

            // Add HTTP configuration related layers
            match config.http() {
                Some(http) => {
                    // Request
                    if let Some(req) = http.request() {
                        if req.trace() {
                            app = app.layer(TraceLayer::new_for_http());
                        }
                        let limit = req.max_request_size().unwrap_or_default();
                        if limit > 10 {
                            app = app.layer(RequestBodyLimitLayer::new(limit));
                        }
                    }

                    // Response
                    // TODO: response middleware
                    #[allow(unused_variables)]
                    if let Some(resp) = http.response() {}
                }
                None => {}
            };

            // Middleware
            app = app
                // Cors
                .layer(
                    CorsLayer::new()
                        .allow_origin(tower_http::cors::Any)
                        .allow_methods(tower_http::cors::Any)
                        .allow_headers(tower_http::cors::Any)
                        .max_age(std::time::Duration::from_secs(60) * 10),
                )
                // Handler request  max timeout
                .layer(TimeoutLayer::new(std::time::Duration::from_secs(
                    req_timeout.unwrap_or(5),
                )))
                // Global panic handler
                .layer(CatchPanicLayer::custom(Self::catch_panic));
        }

        // 6. Trigger the ApplicationReadyEvent
        for ready_event in ctx.resolve_by_type::<Box<dyn ApplicationReadyEvent>>() {
            ready_event.ready(&mut ctx).await;
        }

        // 7. On Ready
        self.on_ready(&mut ctx).await;

        // 8. Add State to [Context]
        app = app.route_layer(axum::Extension(ApplicationState::from_context(ctx)));

        // 9. Nest context path
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
        println!("\nApplication Listening on:  {}", format!("{}:{}", server_addr, server_port));
        println!("Application Started   at:  {}", LocalDateTime::now());
        println!("Application Startup time:  {:?}", time.elapsed());
        println!("Application Process   ID:  {:?}\n", std::process::id());

        // 10. Start server
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

            // Execute all shutdown hooks
            for mut service in shutdowns {
                service.shutdown().await;
            }
        };

        // Configure certificate and private key used by https
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

        // Perform a series of processing on application properties before executing the next step
        next_application
            .application_properties
            .replace_placeholders();

        // Banner show
        Self::banner_show(next_application.application_resources());

        let allow_override = next_application
            .application_properties()
            .next()
            .appliation()
            .map(|s| s.context().allow_override())
            .unwrap_or(false);
        let mut ctx = ApplicationContext::options()
            .allow_override(allow_override)
            .auto_register();

        info!("Init Application context success");

        let mut post_processors = ctx.resolve_by_type::<Box<dyn PropertiesPostProcessor>>();
        post_processors.sort_by_key(|item| item.order());

        post_processors.into_iter().for_each(|mut item| {
            item.post_process_properties(next_application.application_properties.mapping_mut())
        });

        let properties = next_application.application_properties();
        let args = next_application.application_args();
        let resources = next_application.application_resources();

        let application = next_application.application();

        application.init_logging(properties);
        info!("Logging initialized");

        // Autowire properties
        application.autowire_properties(&mut ctx, properties).await;
        info!("Configuration properties loaded");

        // Register singleton
        application
            .register_singleton(&mut ctx, properties, args, resources)
            .await;
        info!("Singleton services registered");

        // Init infrastructure
        application.init_infrastructure(&mut ctx, properties).await;
        info!("Infrastructure initialized",);

        // Init middleware
        application.init_middleware(properties).await;
        info!("Middleware initialized");

        #[cfg(feature = "enable-grpc")]
        {
            application
                .register_rpc_server(&mut ctx, properties, args, resources)
                .await;
            info!("gRPC server started");

            application
                .connect_rpc_client(&mut ctx, properties, args, resources)
                .await;
            info!("gRPC client connected",);
        }

        info!("Starting Async Runtime: [Tokio/1.44.1]");
        info!("Starting HTTP  Server:  [Axum/0.8.4]");

        application
            .bind_tcp_server(ctx, properties, start_time)
            .await;
    }
}
