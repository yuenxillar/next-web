use std::{collections::HashMap, panic::AssertUnwindSafe, sync::Arc, time::Duration};

use futures::{FutureExt, future::join_all};
use next_web_core::traits::service::background_service::{
    BackgroundService, HealthStatus, ServiceError,
};
use tokio::{
    sync::{Mutex, oneshot},
    task::{AbortHandle, JoinHandle},
    time::{MissedTickBehavior, interval, sleep, timeout},
};
use tokio_util::sync::CancellationToken;

use crate::ApplicationShutdownHook;

/// Result of starting or stopping one registered service.
#[derive(Debug)]
pub struct StartedResult {
    pub name: &'static str,
    pub result: Result<(), ServiceError>,
}

/// Current lifecycle state of a managed background service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Stopped,
    Running,
    Failed,
    ShuttingDown,
}

struct ServiceHandle {
    service: Arc<dyn BackgroundService>,
    join_handle: Option<JoinHandle<()>>,
    abort_handle: Option<AbortHandle>,
    state: ServiceState,
}

impl ServiceHandle {
    fn new(service: Arc<dyn BackgroundService>) -> Self {
        Self {
            service,
            join_handle: None,
            abort_handle: None,
            state: ServiceState::Stopped,
        }
    }

    fn is_running(&self) -> bool {
        self.state == ServiceState::Running
            && self
                .join_handle
                .as_ref()
                .is_some_and(|handle| !handle.is_finished())
    }

    fn mark_stopped(&mut self) {
        self.state = ServiceState::Stopped;
        self.join_handle = None;
        self.abort_handle = None;
    }
}

/// Registers, starts, monitors, restarts, and gracefully stops background services.
///
/// Each running service receives periodic health checks. An unhealthy service or a
/// `run` future that exits unexpectedly is shut down and restarted after a short
/// delay. Cancelling the manager stops the supervisor, calls `shutdown`, and gives
/// the service's `run` future time to finish before it is aborted on timeout.
#[derive(Clone)]
pub struct BackgroundServiceManager {
    services: Arc<Mutex<HashMap<&'static str, ServiceHandle>>>,
    shutdown_lock: Arc<Mutex<()>>,
    shutdown_token: CancellationToken,
    shutdown_timeout: Duration,
    heartbeat_interval: Duration,
    restart_delay: Duration,
}

impl BackgroundServiceManager {
    /// Creates a manager with a five-second shutdown timeout, a 30-second
    /// heartbeat interval, and a one-second restart delay.
    #[must_use]
    pub fn new() -> Self {
        Self::with_options(
            Duration::from_secs(5),
            Duration::from_secs(30),
            Duration::from_secs(1),
        )
    }

    /// Creates a manager with a custom shutdown timeout and default heartbeat
    /// and restart intervals.
    #[must_use]
    pub fn with_timeout(timeout: Duration) -> Self {
        Self::with_options(timeout, Duration::from_secs(30), Duration::from_secs(1))
    }

    /// Creates a manager with explicit lifecycle intervals.
    #[must_use]
    pub fn with_options(
        shutdown_timeout: Duration,
        heartbeat_interval: Duration,
        restart_delay: Duration,
    ) -> Self {
        let nonzero = Duration::from_millis(1);
        Self {
            services: Arc::default(),
            shutdown_lock: Arc::new(Mutex::new(())),
            shutdown_token: CancellationToken::new(),
            shutdown_timeout,
            heartbeat_interval: heartbeat_interval.max(nonzero),
            restart_delay: restart_delay.max(nonzero),
        }
    }

    /// Registers a service without starting it.
    ///
    /// # Errors
    /// Returns `AlreadyRunning` if another service uses the same name, or
    /// `Other` if shutdown has already begun.
    pub async fn register(&self, service: Arc<dyn BackgroundService>) -> Result<(), ServiceError> {
        if self.shutdown_token.is_cancelled() {
            return Err(ServiceError::Other("manager is shutting down".to_owned()));
        }

        let service_name = service.name();
        let mut services = self.services.lock().await;
        if self.shutdown_token.is_cancelled() {
            return Err(ServiceError::Other("manager is shutting down".to_owned()));
        }
        if services.contains_key(service_name) {
            return Err(ServiceError::AlreadyRunning);
        }

        services.insert(service_name, ServiceHandle::new(service));
        Ok(())
    }

    /// Registers manager shutdown with an application shutdown hook.
    ///
    /// The callback stops all services and waits for their cleanup, subject to
    /// the manager's configured timeout.
    pub fn register_shutdown_hook(&self, shutdown_hook: &ApplicationShutdownHook) -> usize {
        let manager = self.clone();
        shutdown_hook.add_hook(move || async move {
            for result in manager.shutdown_all().await {
                if let Err(error) = result.result {
                    tracing::error!(service = %result.name, %error, "Failed to stop background service.");
                }
            }
        })
    }

    /// Starts all registered services.
    pub async fn start_all(&self) -> Vec<StartedResult> {
        let service_names: Vec<&'static str> = {
            let services = self.services.lock().await;
            services.keys().copied().collect()
        };

        let starts = service_names.into_iter().map(|name| async move {
            let result = self.start_single(name).await;
            StartedResult { name, result }
        });
        join_all(starts).await
    }

    async fn start_single(&self, name: &'static str) -> Result<(), ServiceError> {
        if self.shutdown_token.is_cancelled() {
            return Err(ServiceError::Other("manager is shutting down".to_owned()));
        }

        let mut services = self.services.lock().await;
        if self.shutdown_token.is_cancelled() {
            return Err(ServiceError::Other("manager is shutting down".to_owned()));
        }

        let handle = services.get_mut(name).ok_or(ServiceError::NotFound)?;
        if handle.is_running() {
            return Err(ServiceError::AlreadyRunning);
        }

        // A finished supervisor may leave its result in the handle. Drop it
        // before creating a fresh one.
        handle.join_handle = None;
        handle.abort_handle = None;

        let service = Arc::clone(&handle.service);
        let (started_sender, started_receiver) = oneshot::channel();
        let join_handle = tokio::spawn(supervise_service(
            service,
            self.shutdown_token.clone(),
            self.shutdown_timeout,
            self.heartbeat_interval,
            self.restart_delay,
            name,
            Some(started_sender),
        ));

        handle.abort_handle = Some(join_handle.abort_handle());
        handle.join_handle = Some(join_handle);
        handle.state = ServiceState::Running;
        drop(services);

        match timeout(self.shutdown_timeout, started_receiver).await {
            Ok(Ok(Ok(()))) => Ok(()),
            Ok(Ok(Err(error))) => Err(error),
            Ok(Err(_)) => Err(ServiceError::InitFailed(
                "service supervisor stopped before startup completed".to_owned(),
            )),
            Err(_) => Err(ServiceError::InitFailed(
                "service did not start before the startup timeout".to_owned(),
            )),
        }
    }

    /// Requests graceful shutdown and waits for every service concurrently.
    pub async fn shutdown_all(&self) -> Vec<StartedResult> {
        let _shutdown_guard = self.shutdown_lock.lock().await;
        self.shutdown_token.cancel();

        let service_names: Vec<&'static str> = {
            let services = self.services.lock().await;
            services.keys().copied().collect()
        };

        let stops = service_names.into_iter().map(|name| async move {
            let result = self.shutdown_single(name).await;
            StartedResult { name, result }
        });
        join_all(stops).await
    }

    async fn shutdown_single(&self, name: &'static str) -> Result<(), ServiceError> {
        let mut services = self.services.lock().await;
        let handle = services.get_mut(name).ok_or(ServiceError::NotFound)?;

        if !handle.is_running() {
            handle.mark_stopped();
            return Ok(());
        }

        handle.state = ServiceState::ShuttingDown;
        let join_handle = handle.join_handle.take().ok_or(ServiceError::NotFound)?;
        let abort_handle = handle.abort_handle.take();
        drop(services);

        // The supervisor allows one timeout for `shutdown` and another for
        // the `run` future to finish. Leave enough time for both phases before
        // aborting the supervisor from the outside.
        let supervisor_timeout = self.shutdown_timeout.saturating_mul(2);
        let result = timeout(supervisor_timeout, join_handle).await;
        let mut services = self.services.lock().await;
        let handle = services.get_mut(name).ok_or(ServiceError::NotFound)?;

        match result {
            Ok(Ok(())) => {
                handle.mark_stopped();
                Ok(())
            }
            Ok(Err(error)) => {
                handle.state = ServiceState::Failed;
                handle.abort_handle = None;
                tracing::error!(service = %name, %error, "Background service supervisor failed.");
                Err(ServiceError::Other(format!(
                    "service supervisor failed: {error}"
                )))
            }
            Err(_) => {
                if let Some(abort_handle) = abort_handle {
                    abort_handle.abort();
                }
                handle.state = ServiceState::Failed;
                handle.join_handle = None;
                handle.abort_handle = None;
                tracing::warn!(service = %name, "Service shutdown timed out; aborting its supervisor.");
                Err(ServiceError::ShutdownTimeout)
            }
        }
    }

    /// Checks one running service. Returns `None` if it is not running.
    pub async fn health_check(&self, name: &str) -> Option<HealthStatus> {
        let (service, running) = {
            let services = self.services.lock().await;
            let handle = services.get(name)?;
            (Arc::clone(&handle.service), handle.is_running())
        };

        if !running {
            return None;
        }

        Some(check_service_health(&service, self.shutdown_timeout).await)
    }

    /// Checks every registered service and returns a status for each one.
    pub async fn health_check_all(&self) -> HashMap<&'static str, HealthStatus> {
        let services: Vec<_> = {
            let services = self.services.lock().await;
            services
                .iter()
                .map(|(&name, handle)| {
                    (
                        name,
                        Arc::clone(&handle.service),
                        handle.is_running(),
                        handle.state,
                    )
                })
                .collect()
        };

        let checks = services
            .into_iter()
            .map(|(name, service, running, state)| async move {
                let status = if running {
                    check_service_health(&service, self.shutdown_timeout).await
                } else {
                    HealthStatus::Unhealthy(format!("Service not running (state: {state:?})"))
                };
                (name, status)
            });
        join_all(checks).await.into_iter().collect()
    }

    /// Checks whether the service supervisor is currently running.
    pub async fn is_running(&self, name: &str) -> bool {
        let services = self.services.lock().await;
        services.get(name).is_some_and(ServiceHandle::is_running)
    }

    /// Returns the current state of every registered service.
    pub async fn get_all_states(&self) -> HashMap<&'static str, ServiceState> {
        let services = self.services.lock().await;
        services
            .iter()
            .map(|(&name, handle)| {
                let state = if handle.state == ServiceState::Running && !handle.is_running() {
                    ServiceState::Failed
                } else {
                    handle.state
                };
                (name, state)
            })
            .collect()
    }

    /// Removes services whose supervisors have stopped.
    pub async fn cleanup_stopped(&self) -> usize {
        let mut services = self.services.lock().await;
        let initial_len = services.len();
        services
            .retain(|_, handle| handle.is_running() || handle.state == ServiceState::ShuttingDown);
        let removed = initial_len - services.len();
        if removed > 0 {
            tracing::info!(removed, "Cleaned up stopped background services.");
        }
        removed
    }

    /// Returns the number of registered services.
    pub async fn service_count(&self) -> usize {
        self.services.lock().await.len()
    }

    /// Returns whether at least one service supervisor is running.
    pub async fn has_running_services(&self) -> bool {
        let services = self.services.lock().await;
        services.values().any(ServiceHandle::is_running)
    }
}

impl Default for BackgroundServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BackgroundServiceManager {
    fn drop(&mut self) {
        // Dropping a temporary clone must not stop services still owned by the
        // application shutdown callback or another manager handle.
        if Arc::strong_count(&self.services) == 1 {
            self.shutdown_token.cancel();
        }
    }
}

async fn supervise_service(
    service: Arc<dyn BackgroundService>,
    shutdown_token: CancellationToken,
    shutdown_timeout: Duration,
    heartbeat_interval: Duration,
    restart_delay: Duration,
    service_name: &'static str,
    mut started_sender: Option<oneshot::Sender<Result<(), ServiceError>>>,
) {
    loop {
        if shutdown_token.is_cancelled() {
            if let Some(started_sender) = started_sender.take() {
                let _ = started_sender.send(Err(ServiceError::Other(
                    "manager is shutting down".to_owned(),
                )));
            }
            break;
        }

        let mut run = Box::pin(AssertUnwindSafe(service.run()).catch_unwind());
        let mut heartbeat = interval(heartbeat_interval);
        heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // The first Tokio interval tick is immediate; wait one full period
        // before the first heartbeat check.
        heartbeat.tick().await;

        enum Exit {
            Shutdown,
            RunCompleted,
            Unhealthy(String),
        }

        // Poll the run future once before observing cancellation. That starts
        // services whose first poll binds a socket, so shutdown cannot signal
        // an unstarted service and then accidentally start it while joining.
        let initial_result = run.as_mut().now_or_never();
        let exit = if let Some(result) = initial_result {
            if let Some(started_sender) = started_sender.take() {
                let error = startup_error(&result, service_name);
                let _ = started_sender.send(Err(error));
            }
            log_run_result(result, service_name);
            Exit::RunCompleted
        } else {
            if let Some(started_sender) = started_sender.take() {
                let _ = started_sender.send(Ok(()));
            }
            loop {
                tokio::select! {
                    biased;
                    _ = shutdown_token.cancelled() => break Exit::Shutdown,
                    result = &mut run => {
                        log_run_result(result, service_name);
                        break Exit::RunCompleted;
                    }
                    _ = heartbeat.tick() => {
                        tokio::select! {
                            biased;
                            _ = shutdown_token.cancelled() => break Exit::Shutdown,
                            health_result = timeout(
                                shutdown_timeout,
                                AssertUnwindSafe(service.health_check()).catch_unwind(),
                            ) => match health_result {
                                Ok(Ok(HealthStatus::Unhealthy(reason))) => break Exit::Unhealthy(reason),
                                Ok(Ok(_)) => {}
                                Ok(Err(payload)) => break Exit::Unhealthy(format!(
                                    "health check panicked: {}",
                                    panic_message(payload.as_ref()),
                                )),
                                Err(_) => break Exit::Unhealthy("health check timed out".to_owned()),
                            }
                        }
                    }
                }
            }
        };

        match exit {
            Exit::Shutdown => {
                shutdown_service(&service, shutdown_timeout, service_name).await;
                match timeout(shutdown_timeout, &mut run).await {
                    Ok(Ok(Ok(()))) => {}
                    Ok(Ok(Err(error))) => {
                        tracing::debug!(service = %service_name, %error, "Service run returned during shutdown.")
                    }
                    Ok(Err(payload)) => {
                        tracing::error!(service = %service_name, panic = %panic_message(payload.as_ref()), "Service panicked while shutting down.")
                    }
                    Err(_) => {
                        tracing::warn!(service = %service_name, "Service run did not finish before the shutdown timeout.")
                    }
                }
                break;
            }
            Exit::RunCompleted => {
                if !shutdown_service(&service, shutdown_timeout, service_name).await {
                    break;
                }
            }
            Exit::Unhealthy(reason) => {
                tracing::warn!(service = %service_name, %reason, "Service heartbeat failed; restarting service.");
                if !shutdown_service(&service, shutdown_timeout, service_name).await {
                    break;
                }
                let run_stopped = match timeout(shutdown_timeout, &mut run).await {
                    Ok(Ok(Ok(()))) => true,
                    Ok(Ok(Err(error))) => {
                        tracing::debug!(service = %service_name, %error, "Service run stopped after an unhealthy heartbeat.");
                        true
                    }
                    Ok(Err(payload)) => {
                        tracing::error!(service = %service_name, panic = %panic_message(payload.as_ref()), "Service panicked while restarting.");
                        true
                    }
                    Err(_) => {
                        tracing::warn!(service = %service_name, "Service run did not stop after an unhealthy heartbeat.");
                        false
                    }
                };
                if !run_stopped {
                    break;
                }
            }
        }

        if shutdown_token.is_cancelled() {
            break;
        }

        tokio::select! {
            _ = shutdown_token.cancelled() => {
                break;
            }
            _ = sleep(restart_delay) => {}
        }
    }
}

fn startup_error(
    result: &Result<Result<(), ServiceError>, Box<dyn std::any::Any + Send>>,
    service_name: &'static str,
) -> ServiceError {
    let reason = match result {
        Ok(Ok(())) => "run exited during startup".to_owned(),
        Ok(Err(error)) => format!("run failed during startup: {error}"),
        Err(payload) => format!(
            "run panicked during startup: {}",
            panic_message(payload.as_ref())
        ),
    };
    ServiceError::InitFailed(format!("service {service_name} {reason}"))
}

fn log_run_result(
    result: Result<Result<(), ServiceError>, Box<dyn std::any::Any + Send>>,
    service_name: &'static str,
) {
    match result {
        Ok(Ok(())) => {
            tracing::warn!(service = %service_name, "Service exited unexpectedly; it will be restarted.")
        }
        Ok(Err(error)) => {
            tracing::error!(service = %service_name, %error, "Service failed; it will be restarted.")
        }
        Err(payload) => {
            tracing::error!(service = %service_name, panic = %panic_message(payload.as_ref()), "Service panicked; it will be restarted.")
        }
    }
}

async fn shutdown_service(
    service: &Arc<dyn BackgroundService>,
    shutdown_timeout: Duration,
    service_name: &'static str,
) -> bool {
    match timeout(
        shutdown_timeout,
        AssertUnwindSafe(service.shutdown()).catch_unwind(),
    )
    .await
    {
        Ok(Ok(())) => true,
        Ok(Err(payload)) => {
            tracing::error!(service = %service_name, panic = %panic_message(payload.as_ref()), "Service shutdown panicked.");
            false
        }
        Err(_) => {
            tracing::warn!(service = %service_name, "Service shutdown callback timed out.");
            false
        }
    }
}

async fn check_service_health(
    service: &Arc<dyn BackgroundService>,
    health_timeout: Duration,
) -> HealthStatus {
    match timeout(
        health_timeout,
        AssertUnwindSafe(service.health_check()).catch_unwind(),
    )
    .await
    {
        Ok(Ok(status)) => status,
        Ok(Err(payload)) => HealthStatus::Unhealthy(format!(
            "health check panicked: {}",
            panic_message(payload.as_ref()),
        )),
        Err(_) => HealthStatus::Unhealthy("health check timed out".to_owned()),
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_owned())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use next_web_core::async_trait;
    use next_web_core::traits::service::background_service::{
        BackgroundService, HealthStatus, ServiceError,
    };
    use tokio::sync::Mutex as AsyncMutex;
    use tokio::time::{Duration, sleep, timeout};
    use tokio_util::sync::CancellationToken;

    use crate::ApplicationShutdownHook;

    use super::BackgroundServiceManager;

    struct TestService {
        name: &'static str,
        stop: AsyncMutex<CancellationToken>,
        starts: AtomicUsize,
        shutdowns: AtomicUsize,
        heartbeats: AtomicUsize,
        unhealthy: bool,
        exits_first_run: bool,
        hangs_health_check: bool,
        shutdown_delay: Duration,
        run_stop_delay: Duration,
    }

    impl TestService {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                stop: AsyncMutex::new(CancellationToken::new()),
                starts: AtomicUsize::new(0),
                shutdowns: AtomicUsize::new(0),
                heartbeats: AtomicUsize::new(0),
                unhealthy: false,
                exits_first_run: false,
                hangs_health_check: false,
                shutdown_delay: Duration::ZERO,
                run_stop_delay: Duration::ZERO,
            }
        }

        fn unhealthy(name: &'static str) -> Self {
            Self {
                unhealthy: true,
                ..Self::new(name)
            }
        }

        fn exiting(name: &'static str) -> Self {
            Self {
                exits_first_run: true,
                ..Self::new(name)
            }
        }

        fn hanging_health_check(name: &'static str) -> Self {
            Self {
                hangs_health_check: true,
                ..Self::new(name)
            }
        }

        fn slow_shutdown(name: &'static str, delay: Duration) -> Self {
            Self {
                shutdown_delay: delay,
                run_stop_delay: delay,
                ..Self::new(name)
            }
        }
    }

    #[async_trait]
    impl BackgroundService for TestService {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn run(&self) -> Result<(), ServiceError> {
            let run_number = self.starts.fetch_add(1, Ordering::SeqCst);
            let stop = {
                let mut current = self.stop.lock().await;
                let token = CancellationToken::new();
                *current = token.clone();
                token
            };
            if self.exits_first_run && run_number == 0 {
                return Ok(());
            }
            stop.cancelled().await;
            sleep(self.run_stop_delay).await;
            Ok(())
        }

        async fn shutdown(&self) {
            self.shutdowns.fetch_add(1, Ordering::SeqCst);
            sleep(self.shutdown_delay).await;
            self.stop.lock().await.cancel();
        }

        async fn health_check(&self) -> HealthStatus {
            self.heartbeats.fetch_add(1, Ordering::SeqCst);
            if self.hangs_health_check {
                return std::future::pending().await;
            }
            if self.unhealthy {
                HealthStatus::Unhealthy("test failure".to_owned())
            } else {
                HealthStatus::Healthy
            }
        }
    }

    async fn wait_for(counter: &AtomicUsize, minimum: usize) {
        timeout(Duration::from_secs(1), async {
            while counter.load(Ordering::SeqCst) < minimum {
                sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("counter should reach the expected value");
    }

    #[tokio::test]
    async fn heartbeats_and_application_hook_stop_a_service_gracefully() {
        let manager = BackgroundServiceManager::with_options(
            Duration::from_millis(200),
            Duration::from_millis(10),
            Duration::from_millis(10),
        );
        let service = Arc::new(TestService::new("test-service"));
        manager.register(service.clone()).await.unwrap();
        let hook = ApplicationShutdownHook::new();
        manager.register_shutdown_hook(&hook);

        let result = manager.start_all().await;
        assert!(result.iter().all(|result| result.result.is_ok()));
        wait_for(&service.starts, 1).await;
        wait_for(&service.heartbeats, 1).await;

        hook.shutdown().await;

        assert_eq!(service.shutdowns.load(Ordering::SeqCst), 1);
        assert!(!manager.is_running("test-service").await);
        assert_eq!(manager.service_count().await, 1);
    }

    #[tokio::test]
    async fn unhealthy_heartbeat_restarts_the_service() {
        let manager = BackgroundServiceManager::with_options(
            Duration::from_millis(200),
            Duration::from_millis(10),
            Duration::from_millis(5),
        );
        let service = Arc::new(TestService::unhealthy("unhealthy-service"));
        manager.register(service.clone()).await.unwrap();
        let results = manager.start_all().await;
        assert!(results.iter().all(|result| result.result.is_ok()));

        wait_for(&service.starts, 2).await;
        assert!(service.shutdowns.load(Ordering::SeqCst) >= 1);
        manager.shutdown_all().await;
    }

    #[tokio::test]
    async fn unexpectedly_exiting_service_is_restarted() {
        let manager = BackgroundServiceManager::with_options(
            Duration::from_millis(200),
            Duration::from_millis(50),
            Duration::from_millis(5),
        );
        let service = Arc::new(TestService::exiting("exiting-service"));
        manager.register(service.clone()).await.unwrap();
        let results = manager.start_all().await;
        assert!(matches!(
            &results[0].result,
            Err(ServiceError::InitFailed(_))
        ));

        wait_for(&service.starts, 2).await;
        assert!(service.shutdowns.load(Ordering::SeqCst) >= 1);
        manager.shutdown_all().await;
    }

    #[tokio::test]
    async fn shutdown_interrupts_a_pending_heartbeat_check() {
        let manager = BackgroundServiceManager::with_options(
            Duration::from_secs(1),
            Duration::from_millis(5),
            Duration::from_millis(5),
        );
        let service = Arc::new(TestService::hanging_health_check("pending-health-check"));
        manager.register(service.clone()).await.unwrap();
        let results = manager.start_all().await;
        assert!(results.iter().all(|result| result.result.is_ok()));
        wait_for(&service.heartbeats, 1).await;

        let stopped = timeout(Duration::from_millis(200), manager.shutdown_all()).await;
        assert!(stopped.is_ok(), "shutdown should not wait for heartbeat timeout");
        assert_eq!(service.shutdowns.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn shutdown_waits_for_both_cleanup_phases() {
        let manager = BackgroundServiceManager::with_options(
            Duration::from_millis(300),
            Duration::from_secs(1),
            Duration::from_millis(5),
        );
        let service = Arc::new(TestService::slow_shutdown(
            "slow-shutdown",
            Duration::from_millis(250),
        ));
        manager.register(service.clone()).await.unwrap();
        let results = manager.start_all().await;
        assert!(results.iter().all(|result| result.result.is_ok()));

        let started = tokio::time::Instant::now();
        let results = timeout(Duration::from_secs(1), manager.shutdown_all())
            .await
            .expect("both bounded cleanup phases should finish");

        assert!(results.iter().all(|result| result.result.is_ok()));
        assert!(started.elapsed() >= Duration::from_millis(450));
        assert_eq!(service.shutdowns.load(Ordering::SeqCst), 1);
    }
}
