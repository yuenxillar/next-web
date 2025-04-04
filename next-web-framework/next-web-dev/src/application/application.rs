use std::fs::{self};
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Bytes;
use axum::http::{Response, StatusCode};
use axum::Router;
use hashbrown::HashMap;
use http_body_util::Full;
use once_cell::sync::Lazy;
use rudi::Context as ApplicationContext;
use rust_embed_for_web::{EmbedableFile, RustEmbed};
use tokio::sync::Mutex;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

use crate::application::application_shutdown::ApplicationShutdown;
use crate::application::next_application::NextApplication;

use crate::event::application_event::ApplicationEvent;
use crate::event::application_event_multicaster::ApplicationEventMulticaster;
use crate::event::application_event_publisher::ApplicationEventPublisher;
use crate::event::application_listener::ApplicationListener;

use crate::autoconfigure::context::next_properties::NextProperties;
use crate::autoregister::register_single::ApplicationDefaultRegisterSingle;
use crate::banner::top_banner::{TopBanner, DEFAULT_TOP_BANNER};
use crate::event::default_application_event_multicaster::DefaultApplicationEventMulticaster;
use crate::router::open_router::OpenRouter;
use crate::router::private_router::PrivateRouter;
use crate::util::date_time_util::LocalDateTimeUtil;
use crate::util::file_util::FileUtil;
use crate::util::sys_path::resources;

use super::application_properties::ApplicationProperties;
use super::application_resources::ApplicationResources;

#[cfg(feature = "job_scheduler")]
use crate::autoregister::job_scheduler_autoregister::JobSchedulerAutoRegister;
#[cfg(feature = "job_scheduler")]
use crate::manager::job_scheduler_manager::{ApplicationJob, JobSchedulerManager};

#[cfg(feature = "redis_enabled")]
use crate::event::redis_expired_event::RedisExpiredEvent;
#[cfg(feature = "redis_enabled")]
use crate::manager::redis_manager::RedisManager;

pub const APPLICATION_BANNER_NAME: &str = "banner.txt";
pub const APPLICATION_USER_PERMISSION_RESOURCE: &str = "user_permission_resource.json";

/// The application shutdown services.
#[cfg(not(feature = "tls_rustls"))]
static SHUTDOWN_SERVICES: Lazy<Mutex<Vec<Arc<dyn ApplicationShutdown>>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

#[async_trait]
pub trait Application: Send + Sync {
    /// initialize the middleware.
    async fn init_middleware(&mut self, properties: &ApplicationProperties);

    /// register the rpc server.
    #[cfg(feature = "grpc_enabled")]
    async fn register_rpc_server(&mut self, properties: &ApplicationProperties);

    /// register the grpc client.
    #[cfg(feature = "grpc_enabled")]
    async fn connect_rpc_client(&mut self, properties: &ApplicationProperties);

    /// show the banner of the application.
    fn banner_show() {
        if let Some(content) = ApplicationResources::get(APPLICATION_BANNER_NAME) {
            TopBanner::show(std::str::from_utf8(&content.data()).unwrap_or(DEFAULT_TOP_BANNER));
        } else {
            TopBanner::show(DEFAULT_TOP_BANNER);
        }
    }

    async fn register_services(&mut self, properties: &ApplicationProperties) {
        
    }

    /// initialize the message source.
    async fn init_message_source<T>(
        &mut self,
        application_properties: &NextProperties,
    ) -> HashMap<String, HashMap<String, String>> {
        // message source
        let mut messages = HashMap::new();
        if let Some(message_source) = application_properties.messages() {
            let path = PathBuf::from(resources())
                .join(message_source.basename().unwrap_or_else(|| "/messages"));
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let var = entry.path();
                        let path = var.to_string_lossy();

                        let var3 = FileUtil::read_file_to_string(&path).map(|msg| {
                            if !msg.is_empty() {
                                let mut map = HashMap::new();
                                let _ = msg.lines().into_iter().map(|var1| {
                                    let var2: Vec<&str> = var1.split("=").collect();
                                    if var2.len() == 2 {
                                        let key = var2.get(0).unwrap();
                                        let value = var2.get(1).unwrap();
                                        map.insert(key.to_string(), value.to_string());
                                    }
                                });
                                return map;
                            } else {
                                HashMap::with_capacity(0)
                            }
                        });

                        if let Ok(var4) = var3 {
                            if let Some(name) = var.file_name() {
                                messages.insert(
                                    name.to_string_lossy().to_string().to_lowercase(),
                                    var4,
                                );
                            }
                        }
                    };
                }
            }
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

    /// initialize the application infrastructure
    fn init_infrastructure(
        &self,
        ctx: &mut ApplicationContext,
        application_properties: &ApplicationProperties,
    ) {
        println!("\n========================================================================");

        // 1. register singleton
        self.register_singleton(ctx, application_properties);


        // 2. register job
        #[cfg(feature = "job_scheduler")]
        if let Some(mut schedluer_manager) =
            ctx.resolve_option_with_name::<JobSchedulerManager>("jobSchedulerManager")
        {
            for ele in ctx.resolve_by_type::<Arc<dyn ApplicationJob>>().iter() {
                schedluer_manager.add_job(ele.generate_job());
            }
            schedluer_manager.start();
        } else {
            info!("Job scheduler manager not found");
        }

        // 3. register redis expired event
        #[cfg(feature = "redis_enabled")]
        if let Some(redis_manager) = ctx.resolve_option_with_name::<RedisManager>("redisManager") {
            if let Some(handle) =
                ctx.resolve_option::<Arc<tokio::sync::Mutex<dyn RedisExpiredEvent>>>()
            {
                tokio::task::spawn(async move {
                    let _ = redis_manager
                        .expired_event(handle)
                        .await
                        .map(|_| info!("Redis expired event listen success!"));
                });
            }
        }

        // 3. register application event
        let listeners = ctx.resolve_by_type::<Box<dyn ApplicationListener>>();
        let publishers = ctx.resolve_by_type::<Box<dyn ApplicationEventPublisher>>();

        let event_match =
            |listener: (std::any::TypeId, &str), publisher: (std::any::TypeId, &str)| -> bool {
                return listener.0 == publisher.0 && listener.1.eq(publisher.1);
            };

        let mut multicaster = DefaultApplicationEventMulticaster::new();

        listeners.into_iter().for_each(|listener| {
            multicaster.add_application_listener(listener);
        });
 
        ctx.insert_singleton_with_name::<DefaultApplicationEventMulticaster, String>(multicaster, String::from("defaultApplicationEventMulticaster"));


        println!("========================================================================\n");
    }

    /// register application singleton
    fn register_singleton(
        &self,
        ctx: &mut ApplicationContext,
        application_properties: &ApplicationProperties,
    ) {
        // register register singleton
        application_properties.next().data().map(|data| {
            for element in data.registrable() {
                if let Some(auto_register) = element {
                    auto_register
                        .register(ctx)
                        .map_err(|e| {
                            error!(
                                "Application Data register error, name: <{}>, error: {}",
                                auto_register.name(),
                                e.to_string()
                            )
                        })
                        .unwrap();
                }
            }
        });

        // register application singleton
        let mut container = ApplicationDefaultRegisterSingle::new();
        #[cfg(feature = "job_scheduler")]
        container.push::<JobSchedulerAutoRegister>();

        container.register_all(ctx);
    }

    // get the application router. (open api  and private api)

    async fn applicatlion_router(
        &self,
        context: &ApplicationContext,
    ) -> (OpenRouter, PrivateRouter);

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

    #[cfg(not(feature = "tls_rustls"))]
    fn graceful_shutdown(&self, ctx: &mut ApplicationContext) {
        info!("Graceful Shutdown Start");
        // By Order
        let services = ctx.resolve_by_type::<Arc<dyn ApplicationShutdown>>();
        if let Ok(mut container) = SHUTDOWN_SERVICES.try_lock() {
            container.extend(services);
        }
    }

    /// bind tcp server.
    async fn bind_tcp_server(
        &self,
        application_properties: &ApplicationProperties,
        context: &ApplicationContext,
        time: std::time::Instant,
    ) {
        let config = application_properties.next().server();

        let (open_router, private_router) = self.applicatlion_router(context).await;
        // run our app with hyper, listening globally on port
        let mut app = Router::new()
            .route("/", axum::routing::get(root))
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
        if !config.context_path().is_empty() {
            app = app.nest(config.context_path(), router);
        } else {
            app = app.merge(router);
        }

        println!(
            "\napplication listening on: [{}]",
            format!("0.0.0.0:{}", config.port())
        );

        println!("application start time: {:?}", time.elapsed());

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
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port()))
                .await
                .unwrap();

            axum::serve(listener, app.into_make_service())
                .with_graceful_shutdown(shutdown_signal())
                .await
                .unwrap();
        }
    }

    /// run the application.
    async fn run() -> NextApplication<Self>
    where
        Self: Application + Default,
    {
        // record application start time
        let start_time = std::time::Instant::now();

        // banner show
        Self::banner_show();

        // get a base applcation instance
        let mut next_application: NextApplication<Self> = NextApplication::new();
        let properties = next_application.application_properties().clone();

        let application = next_application.application();

        application.init_logger(&properties);
        info!("init logger success");

        let mut ctx = ApplicationContext::options()
            .eager_create(true)
            .auto_register();

        // init infrastructure
        application.init_infrastructure(&mut ctx, &properties);

        // init middleware
        application.init_middleware(&properties).await;
        info!("init middleware success");

        #[cfg(feature = "grpc_enabled")]
        {
            application.register_rpc_server(&properties).await;
            info!("register rpc server success");

            application.connect_rpc_client(&properties).await;
            info!("connect rpc client success");
        }

        // application.init_cache().await;
        application
            .bind_tcp_server(&properties, &mut ctx, start_time)
            .await;

        next_application
    }
}

fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> Response<Full<Bytes>> {
    error!("Http server handle panic: {:?}", err);
    let err_str = r#"
{
    "status": 500,
    "message": "Internal Server Error",
    "data": null
}"#;
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "application/json")
        .body(Full::from(err_str))
        .unwrap()
}

/// no route match handler
async fn fall_back() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found Route")
}

/// basic handler that responds with a static string
async fn root() -> axum::response::Html<&'static str> {
    axum::response::Html("<html><body><h1>Welcome to Rust Web</h1></body></html>")
}

#[cfg(not(feature = "tls_rustls"))]
async fn shutdown_signal() {
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
            for service in SHUTDOWN_SERVICES.lock().await.iter() {
                service.shutdown().await;
            }

        },
        _ = terminate =>  {
            info!("Received terminate signal. Shutting down...");
            for service in SHUTDOWN_SERVICES.lock().await.iter() {
                service.shutdown().await;
            }
        },
    }
}
