use async_trait::async_trait;
use axum::body::Bytes;
use axum::http::{Response, StatusCode};
use axum::{Router, ServiceExt};
use hashbrown::HashMap;
use http_body_util::Full;
use next_web_core::constants::application_constants::APPLICATION_BANNER_FILE;
use next_web_core::context::application_args::ApplicationArgs;
use next_web_core::context::application_context::ApplicationContext;
use next_web_core::context::properties::{ApplicationProperties, Properties};
use next_web_core::core::router::ApplyRouter;
use next_web_core::AutoRegister;
use rust_embed_for_web::{EmbedableFile, RustEmbed};
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

use crate::application::application_shutdown::ApplicationShutdown;
use crate::application::next_application::NextApplication;

use crate::autoregister::register_single::ApplicationDefaultRegisterContainer;
use crate::event::application_event_multicaster::ApplicationEventMulticaster;
use crate::event::application_listener::ApplicationListener;

use crate::autoconfigure::context::next_properties::NextProperties;
use crate::banner::top_banner::{TopBanner, DEFAULT_TOP_BANNER};
use crate::event::default_application_event_multicaster::DefaultApplicationEventMulticaster;
use crate::event::default_application_event_publisher::DefaultApplicationEventPublisher;
use crate::router::open_router::OpenRouter;
use crate::router::private_router::PrivateRouter;
use crate::util::date_time_util::LocalDateTimeUtil;
use crate::util::file_util::FileUtil;

use next_web_core::context::application_resources::ApplicationResources;

#[cfg(feature = "job_scheduler")]
use crate::manager::job_scheduler_manager::{ApplicationJob, JobSchedulerManager};

#[cfg(feature = "redis_enabled")]
use crate::event::redis_expired_event::RedisExpiredEvent;
#[cfg(feature = "redis_enabled")]
use crate::manager::redis_manager::RedisManager;

pub const APPLICATION_USER_PERMISSION_RESOURCE: &str = "user_permission_resource.json";

#[async_trait]
pub trait Application: Send + Sync {
    /// initialize the middleware.
    async fn init_middleware(&mut self, properties: &ApplicationProperties);

    // get the application router. (open api  and private api)
    async fn application_router(
        &mut self,
        ctx: &mut ApplicationContext,
    ) -> (OpenRouter, PrivateRouter);

    /// register the rpc server.
    #[cfg(feature = "grpc_enabled")]
    async fn register_rpc_server(&mut self, properties: &ApplicationProperties);

    /// register the grpc client.
    #[cfg(feature = "grpc_enabled")]
    async fn connect_rpc_client(&mut self, properties: &ApplicationProperties);

    /// show the banner of the application.
    fn banner_show() {
        if let Some(content) = ApplicationResources::get(APPLICATION_BANNER_FILE) {
            TopBanner::show(std::str::from_utf8(&content.data()).unwrap_or(DEFAULT_TOP_BANNER));
        } else {
            TopBanner::show(DEFAULT_TOP_BANNER);
        }
    }

    /// initialize the message source.
    async fn init_message_source<T>(
        &mut self,
        application_properties: &NextProperties,
    ) -> HashMap<String, HashMap<String, String>> {
        // message source
        let mut messages = HashMap::new();
        if let Some(message_source) = application_properties.messages() {
            let mut load_local_message = |name: &str| {
                if let Some(dyn_file) = ApplicationResources::get(&format!("messages/{}", &name)) {
                    let data = dyn_file.data();
                    if !data.is_empty() {
                        let mut map = HashMap::new();
                        let _ = data.lines().map(|var| {
                            var.map(|var1| {
                                let var2: Vec<&str> = var1.split("=").collect();
                                if var2.len() == 2 {
                                    let key = var2.get(0).unwrap();
                                    let value = var2.get(1).unwrap();
                                    map.insert(key.to_string(), value.to_string());
                                }
                            })
                        });
                        messages.insert(name.to_string(), map);
                    }
                }
            };

            // load default messages
            load_local_message(&"messages.properties");

            message_source.local().map(|item| {
                item.trim_end().split(",").for_each(|s| {
                    let name = format!("messages_{}.properties", s);
                    load_local_message(&name);
                });
            });
        }
        messages
    }

    #[cfg(feature = "user_security")]
    fn user_permission_resource(
        &self,
    ) -> Option<crate::security::user_permission_resource::UserPermissionResource> {
        use crate::security::user_permission_resource::UserPermissionResourceBuilder;

        let path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join(APPLICATION_USER_PERMISSION_RESOURCE)
            .display()
            .to_string();
        if let Ok(content) = FileUtil::read_file_to_string(&path) {
            if content.is_empty() {
                return None;
            }
            let user_permission_resource: Vec<UserPermissionResourceBuilder> =
                serde_json::from_str(&content).unwrap();
            if user_permission_resource.is_empty() {
                return None;
            }
            return Some(user_permission_resource.into());
        }
        None
    }

    /// initialize the logger.
    fn init_logger(&self, application_properties: &ApplicationProperties) {
        let application_name = application_properties
            .next()
            .appliation()
            .map(|app| app.name())
            .unwrap_or_default();
        let file_appender = application_properties.next().logger().map_or_else(
            || None,
            |logger| {
                // logger enable?
                if logger.enable() {
                    let path = logger.log_dir().unwrap_or_else(|| "./logs");
                    let log_name = format!(
                        "{}{}.log",
                        application_name,
                        if logger.additional_date() {
                            format!("-{}", LocalDateTimeUtil::date())
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
            .with_max_level(tracing::Level::INFO)
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
    ) {
        // register application singleton
        let mut container = ApplicationDefaultRegisterContainer::new();
        container.register_all(ctx, application_properties).await;

        // register singletion with properties and args
        ctx.insert_singleton_with_name(application_properties.clone(), "");
        ctx.insert_singleton_with_name(application_args.clone(), "");

        // resove autoRegister
        let auto_register = ctx.resolve_by_type::<Arc<dyn AutoRegister>>();
        for item in auto_register.iter() {
            item.register(ctx, application_properties).await.unwrap();
        }
    }

    /// initialize the application infrastructure
    async fn init_infrastructure(
        &self,
        ctx: &mut ApplicationContext,
        _application_properties: &ApplicationProperties,
    ) {
        // Register job
        let producers = ctx.resolve_by_type::<Arc<dyn ApplicationJob>>();
        if let Some(schedluer_manager) =
            ctx.get_single_option_with_name::<JobSchedulerManager>("jobSchedulerManager")
        {
            let mut schedluer_manager = schedluer_manager.clone();
            for producer in producers {
                schedluer_manager.add_job(producer.gen_job()).await;
            }
            schedluer_manager.start();
        } else {
            warn!("Job scheduler manager not found");
        }

        // Register redis expired event
        #[cfg(feature = "redis_enabled")]
        if let Some(redis_manager) = ctx.resolve_option_with_name::<RedisManager>("redisManager") {
            if let Some(handle) =
                ctx.resolve_option::<Arc<tokio::sync::Mutex<dyn RedisExpiredEvent>>>()
            {
                let _ = redis_manager
                    .expired_event(handle)
                    .await
                    .map(|_| info!("Redis expired event listen success!"));
            }
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

        ctx.insert_singleton_with_name(default_event_publisher, "");
        ctx.insert_singleton_with_name(multicaster, "");
    }

    /// bind tcp server.
    async fn bind_tcp_server(
        &mut self,
        ctx: &mut ApplicationContext,
        application_properties: &ApplicationProperties,
        time: std::time::Instant,
    ) {
        let config = application_properties.next().server();

        let (open_router, private_router) = self.application_router(ctx).await;
        // run our app with hyper, listening globally on port
        let mut app = Router::new()
            .merge(Router::new().nest("/open", open_router.0))
            // handle not found route
            .fallback(fall_back);

        // add prometheus layer
        #[cfg(feature = "prometheus_enabled")]
        {
            let (prometheus_layer, metric_handle) = axum_prometheus::PrometheusMetricLayer::pair();
            app = app
                .route(
                    "/metrics",
                    axum::routing::get(|| async move { metric_handle.render() }),
                )
                .layer(prometheus_layer);
        }

        let mut router = private_router.0;
        // set layer
        if let Some(http) = config.http() {
            let val = http
                .request()
                .map(|req| {
                    let var1 = req.trace();
                    let var2 = req.max_request_size().unwrap_or_default();
                    (var1, var2)
                })
                .unwrap_or_default();
            let _ = http.response();
            if val.0 {
                router = router.layer(TraceLayer::new_for_http());
            }
            // 3MB
            if val.1 >= 3145728 {
                router = router.route_layer(RequestBodyLimitLayer::new(val.1));
            }
        }

        router = router
            // global panic handler
            .layer(CatchPanicLayer::custom(handle_panic))
            // handler request  max timeout
            .layer(TimeoutLayer::new(std::time::Duration::from_secs(5)))
            // cors  pass -> anyeventing request
            .layer(
                CorsLayer::new()
                    .allow_origin(tower_http::cors::Any)
                    .allow_methods(tower_http::cors::Any)
                    .allow_headers(tower_http::cors::Any)
                    .max_age(std::time::Duration::from_secs(60) * 10),
            );
        // #[cfg(feature = "user_security")]
        // router = router.layer(axum::middleware::from_fn_with_state(crate::security::request_auth_middleware));
        if let Some(path) = config.context_path() {
            app = app.nest(path, router);
        } else {
            app = app.merge(router);
        }

        // apply router
        let routers = ctx.resolve_by_type::<Box<dyn ApplyRouter>>();
        for item in routers.into_iter() {
            let router = item.router(ctx);
            if !router.has_routes() { continue; }
            if let Some(path) = config.context_path() {
                if !path.is_empty() {
                    app = app.nest(path, router);
                    continue;
                }
            }
            app = app.merge(router);
        }

        let server_port = config.port().unwrap_or(8080);
        let server_addr = if config.local().unwrap_or(false) {
            "127.0.0.1"
        } else {
            "0.0.0.0"
        };
        println!(
            "\napplication listening  on:  {}",
            format!("{}:{}", server_addr, server_port)
        );

        println!("application running    on:  {}", LocalDateTimeUtil::now());
        println!("application startTime  on:  {:?}", time.elapsed());
        println!("application currentPid on:  {:?}\n", std::process::id());

        // configure certificate and private key used by https
        #[cfg(feature = "tls_rustls")]
        {
            let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("self_signed_certs")
                    .join("cert.pem"),
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("self_signed_certs")
                    .join("key.pem"),
            )
            .await
            .unwrap();
            let addr = std::net::SocketAddr::from(([0, 0, 0, 0], config.port()));
            let mut server = axum_server::bind_rustls(addr, tls_config);
            // IMPORTANT: This is required to advertise our support for HTTP/2 websockets to the client.
            // If you use axum::serve, it is enabled by default.
            server.http_builder().http2().enable_connect_protocol();
            server.serve(app.into_make_service()).await.unwrap();
        }

        #[cfg(not(feature = "tls_rustls"))]
        {
            // run http server
            let listener =
                tokio::net::TcpListener::bind(format!("{}:{}", server_addr, server_port))
                    .await
                    .unwrap();

            let mut shutdowns = ctx.resolve_by_type::<Box<dyn ApplicationShutdown>>();
            axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
                .with_graceful_shutdown(async move {
                    let ctrl_c = async {
                        tokio::signal::ctrl_c()
                            .await
                            .expect("failed to install Ctrl+C handler");
                    };

                    #[cfg(unix)]
                    let terminate = async {
                        signal::unix::signal(signal::unix::SignalKind::terminate())
                            .expect("failed to install signal handler")
                            .recv()
                            .await;
                    };

                    #[cfg(not(unix))]
                    let terminate = std::future::pending::<()>();

                    tokio::select! {
                        _ = ctrl_c =>  {
                            info!("Received Ctrl+C. Shutting down...");
                            for service in shutdowns.iter_mut() {
                                service.shutdown().await;
                            }
                        },
                        _ = terminate =>  {
                            info!("Received terminate signal. Shutting down...");
                            for service in shutdowns.iter_mut() {
                                service.shutdown().await;
                            }
                        },
                    }
                })
                .await
                .unwrap();
        }
    }

    /// run the application.
    async fn run()
    where
        Self: Application + Default,
    {
        // record application start time
        let start_time = std::time::Instant::now();

        // banner show
        Self::banner_show();

        // get a base application instance
        let mut next_application: NextApplication<Self> = NextApplication::new();
        let properties = next_application.application_properties().clone();
        let args = next_application.application_args().clone();

        let application = next_application.application();

        println!("========================================================================\n");

        application.init_logger(&properties);
        println!(
            "init logger success!\ncurrent time: {}\n",
            LocalDateTimeUtil::now()
        );

        let mut ctx = ApplicationContext::options()
            .allow_override(false)
            .auto_register();
        println!(
            "init application context success!\ncurrent time: {}\n",
            LocalDateTimeUtil::now()
        );

        // autowire properties
        application.autowire_properties(&mut ctx, &properties).await;
        println!(
            "autowire properties success!\ncurrent time: {}\n",
            LocalDateTimeUtil::now()
        );

        // register singleton
        application
            .register_singleton(&mut ctx, &properties, &args)
            .await;
        println!(
            "register singleton success!\ncurrent time: {}\n",
            LocalDateTimeUtil::now()
        );

        // init infrastructure
        application.init_infrastructure(&mut ctx, &properties).await;
        println!(
            "init infrastructure success!\ncurrent time: {}\n",
            LocalDateTimeUtil::now()
        );

        // init middleware
        application.init_middleware(&properties).await;
        println!(
            "init middleware success!\ncurrent time: {}\n",
            LocalDateTimeUtil::now()
        );

        #[cfg(feature = "grpc_enabled")]
        {
            application.register_rpc_server(&properties).await;
            println!(
                "register grpc server success!\ncurrent time: {}\n",
                LocalDateTimeUtil::now()
            );

            application.connect_rpc_client(&properties).await;
            println!(
                "connect grpc client success!\ncurrent time: {}\n",
                LocalDateTimeUtil::now()
            );
        }

        println!("========================================================================");

        // application.init_cache().await;

        application
            .bind_tcp_server(&mut ctx, &properties, start_time)
            .await;
    }
}

fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> Response<Full<Bytes>> {
    error!("application handle panic, case: {:?}", err);
    let err_str = format!(
        "internal server error, case: {:?},\ntimestamp: {}",
        err,
        LocalDateTimeUtil::timestamp()
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
