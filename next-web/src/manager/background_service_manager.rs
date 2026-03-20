use std::{collections::HashMap, sync::Arc, time::Duration};

use next_web_core::traits::service::background_service::{
    BackgroundService, HealthStatus, ServiceError,
};
use tokio::{
    sync::{
        broadcast::{self, Sender},
        Mutex,
    },
    task::{AbortHandle, JoinHandle},
    time::timeout,
};

/// Service startup result
///
/// 服务启动结果
#[derive(Debug)]
pub struct StartedResult {
    pub name: &'static str,
    pub result: Result<(), ServiceError>,
}

/// Service runtime state
///
/// 服务运行时状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Stopped,
    Running,
    Failed,
    ShuttingDown,
}

/// Internal handle that encapsulates service and its runtime information
///
/// 内部管理句柄，封装服务及其运行时信息
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

    /// Check if service is actually running (considers task completion)
    ///
    /// 检查服务是否实际运行中（考虑任务完成状态）
    fn is_running(&self) -> bool {
        if self.state != ServiceState::Running {
            return false;
        }

        // If task is finished but state is still Running, it's not actually running
        //
        // 如果任务已完成但状态仍是Running，则实际未运行
        match &self.join_handle {
            Some(handle) => !handle.is_finished(),
            None => false,
        }
    }

    /// Mark service as stopped and clean up handles
    ///
    /// 标记服务为已停止并清理句柄
    fn mark_stopped(&mut self) {
        self.state = ServiceState::Stopped;
        self.join_handle = None;
        self.abort_handle = None;
    }
}

/// Background Service Manager - Responsible for lifecycle management, health checks, and graceful shutdown
///
/// 后台服务管理器 - 负责生命周期管理、健康检查和优雅关闭
#[derive(Clone)]
pub struct BackgroundServiceManager {
    /// Registered services collection
    ///
    /// 已注册的服务集合
    services: Arc<Mutex<HashMap<&'static str, ServiceHandle>>>,

    /// Global shutdown signal broadcast channel
    ///
    /// 全局关闭信号广播通道
    shutdown_sender: Sender<()>,

    /// Shutdown timeout configuration
    ///
    /// 关闭超时配置
    shutdown_timeout: Duration,
}

impl BackgroundServiceManager {
    /// Create a new manager instance (default timeout: 5 seconds)
    ///
    /// 创建新的管理器实例（默认超时5秒）
    #[must_use]
    pub fn new() -> Self {
        Self::with_timeout(Duration::from_secs(5))
    }

    /// Create manager with custom shutdown timeout
    ///
    /// 创建管理器并自定义关闭超时时间
    #[must_use]
    pub fn with_timeout(timeout: Duration) -> Self {
        let (shutdown_sender, _) = broadcast::channel(1); // Only need buffer for 1 signal / 只需缓冲1个信号
        Self {
            shutdown_sender,
            services: Arc::default(),
            shutdown_timeout: timeout,
        }
    }

    /// Register a new service (service not started)
    ///
    /// # Errors
    /// Returns `ServiceError::AlreadyRunning` if service name already exists
    ///
    /// 注册一个新服务（服务未启动）
    ///
    /// 返回 `ServiceError::AlreadyRunning` 如果服务名已存在
    pub async fn register(&self, service: Arc<dyn BackgroundService>) -> Result<(), ServiceError> {
        let service_name = service.service_name();
        let mut services = self.services.lock().await;

        if services.contains_key(service_name) {
            return Err(ServiceError::AlreadyRunning);
        }

        services.insert(service_name, ServiceHandle::new(service));
        Ok(())
    }

    /// Start all registered services
    /// Returns startup result for each service, failed services will be marked as `Failed` state
    ///
    /// 启动所有已注册的服务
    /// 返回每个服务的启动结果，失败的服务会被标记为 `Failed` 状态
    #[tracing::instrument(skip(self), fields(services_count))]
    pub async fn start_all(&self) -> Vec<StartedResult> {
        // 1. Quickly get service snapshot to reduce lock holding time
        // 快速获取服务快照，减少锁持有时间
        let service_names: Vec<&'static str> = {
            let services = self.services.lock().await;
            services.keys().copied().collect()
        };

        if service_names.is_empty() {
            return Vec::new();
        }

        tracing::Span::current().record("services_count", service_names.len());

        // 2. Start all services
        // 并行启动所有服务
        let tasks: Vec<_> = service_names
            .iter()
            .map(|&name| async move {
                let result = self.start_single(name).await;
                (name, result)
            })
            .collect();

        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            let (name, result) = task.await;
            results.push(StartedResult { name, result });
        }

        results
    }

    /// Start a single service (internal use)
    ///
    /// 启动单个服务（内部使用）
    async fn start_single(&self, name: &'static str) -> Result<(), ServiceError> {
        let mut services = self.services.lock().await;
        let handle = services.get_mut(name).ok_or(ServiceError::NotFound)?;

        // Check if already running
        // 检查是否已在运行
        if handle.is_running() {
            return Err(ServiceError::AlreadyRunning);
        }

        // Clean up any existing handles before starting
        // 启动前清理任何现有句柄
        handle.join_handle = None;
        handle.abort_handle = None;

        // Prepare service startup
        // 准备服务启动
        let service = Arc::clone(&handle.service);
        let mut shutdown_rx = self.shutdown_sender.subscribe();
        let service_name = name; // For closure capture

        let join_handle = tokio::spawn(async move {
            tokio::select! {
                // Service normal execution
                result = service.run() => {
                    if let Err(e) = result {
                        tracing::error!(service = %service_name, error = %e, "Service execution error.");
                    } else {
                        tracing::debug!(service = %service_name, "Service exited normally.");
                    }
                }
                // Received shutdown signal
                _ = shutdown_rx.recv() => {
                    tracing::debug!(service = %service_name, "Received shutdown signal, executing graceful shutdown.");
                    service.shutdown().await;
                }
            }
        });

        // Update handle state
        // 更新句柄状态
        handle.abort_handle = Some(join_handle.abort_handle());
        handle.join_handle = Some(join_handle);
        handle.state = ServiceState::Running;

        Ok(())
    }

    /// Gracefully shut down all services
    ///
    /// Send a global shutdown signal once, wait for all services to complete cleanup within timeout
    ///
    /// 优雅关闭所有服务
    ///
    /// 发送一次全局关闭信号，等待所有服务在超时时间内完成清理
    #[tracing::instrument(skip(self))]
    pub async fn shutdown_all(&self) -> Vec<StartedResult> {
        // 1. Send single shutdown signal (broadcast to all subscribers)
        // 发送单次关闭信号（广播给所有订阅者）
        let _ = self.shutdown_sender.send(());

        // 2. Get service snapshot
        // 获取服务快照
        let service_names: Vec<&'static str> = {
            let services = self.services.lock().await;
            services.keys().copied().collect()
        };

        if service_names.is_empty() {
            return Vec::new();
        }

        // 3. Wait for all services to shut down in parallel
        // 并行等待所有服务关闭
        let tasks: Vec<_> = service_names
            .iter()
            .map(|&name| {
                let this = self.clone();
                tokio::spawn(async move {
                    let result = this.shutdown_single(name).await;
                    (name, result)
                })
            })
            .collect();

        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            match task.await {
                Ok((name, result)) => results.push(StartedResult { name, result }),
                Err(e) => {
                    tracing::error!("Failed to join shutdown task: {}", e);
                }
            }
        }
        results
    }

    /// Shut down a single service (internal use)
    ///
    /// 关闭单个服务（内部使用）
    async fn shutdown_single(&self, name: &'static str) -> Result<(), ServiceError> {
        let mut services = self.services.lock().await;
        let handle = services.get_mut(name).ok_or(ServiceError::NotFound)?;

        // If not running, return success directly
        // 如果未运行，直接返回成功
        if !handle.is_running() {
            handle.mark_stopped();
            return Ok(());
        }

        // Mark state
        // 标记状态
        handle.state = ServiceState::ShuttingDown;

        // Take the handle to wait for completion
        // 取出句柄等待完成
        if let Some(join_handle) = handle.join_handle.take() {
            // Use a separate timeout to avoid blocking the lock
            // 使用单独的超时避免阻塞锁
            drop(services);

            let result = timeout(self.shutdown_timeout, join_handle).await;

            // Re-acquire lock to update state
            // 重新获取锁以更新状态
            let mut services = self.services.lock().await;
            let handle = services.get_mut(name).ok_or(ServiceError::NotFound)?;

            match result {
                Ok(_) => {
                    handle.mark_stopped();
                    Ok(())
                }
                Err(_) => {
                    tracing::warn!(service = %name, "Service shutdown timeout, forcing termination.");
                    // Force abort after timeout
                    // 超时后强制 abort
                    if let Some(abort) = handle.abort_handle.take() {
                        abort.abort();
                    }
                    handle.state = ServiceState::Failed;
                    handle.join_handle = None;
                    Err(ServiceError::ShutdownTimeout)
                }
            }
        } else {
            handle.mark_stopped();
            Ok(())
        }
    }

    /// Check health status of a single service
    ///
    /// Returns `None` if service is not running or task has crashed
    ///
    /// 检查单个服务的健康状态
    ///
    /// 如果服务未运行或任务已崩溃，返回 `None`
    pub async fn health_check(&self, name: &str) -> Option<HealthStatus> {
        let services = self.services.lock().await;
        let handle = services.get(name)?;

        // If task is finished but state is still Running, it may have panicked
        // 如果任务已 finished 但状态仍是 Running，说明可能 panic 了
        if handle.state == ServiceState::Running {
            if let Some(join_handle) = &handle.join_handle {
                if join_handle.is_finished() {
                    return Some(HealthStatus::Unhealthy(
                        "Task panicked or completed unexpectedly.".into(),
                    ));
                }
            }
        }

        Some(handle.service.health_check().await)
    }

    /// Batch check health status of all services
    ///
    /// 批量检查所有服务的健康状态
    pub async fn health_check_all(&self) -> HashMap<&'static str, HealthStatus> {
        let services = self.services.lock().await;
        let mut results = HashMap::with_capacity(services.len());

        for (&name, handle) in services.iter() {
            let status = if handle.is_running() {
                handle.service.health_check().await
            } else {
                HealthStatus::Unhealthy(format!("Service not running  (state: {:?})", handle.state))
            };
            results.insert(name, status);
        }
        results
    }

    /// Check if service is running (considers actual task status)
    ///
    /// 检查服务是否正在运行（考虑任务实际状态）
    pub async fn is_running(&self, name: &str) -> bool {
        let services = self.services.lock().await;
        services.get(name).map_or(false, |h| h.is_running())
    }

    /// Get snapshot of all services' current states
    ///
    /// 获取所有服务的当前状态快照
    pub async fn get_all_states(&self) -> HashMap<&'static str, ServiceState> {
        let services = self.services.lock().await;
        services
            .iter()
            .map(|(&name, handle)| (name, handle.state))
            .collect()
    }

    /// Remove stopped services (clean up resources)
    ///
    /// 移除已停止的服务（清理资源）
    pub async fn cleanup_stopped(&self) -> usize {
        let mut services = self.services.lock().await;
        let initial_len = services.len();

        services.retain(|_, handle| {
            // Keep services that are running or have unfinished handles
            // 保留正在运行或句柄还未完成的
            if handle.state == ServiceState::Running {
                // Double-check with join_handle
                // 用 join_handle 双重检查
                if let Some(join_handle) = &handle.join_handle {
                    !join_handle.is_finished()
                } else {
                    false
                }
            } else {
                false
            }
        });

        let removed = initial_len - services.len();
        if removed > 0 {
            tracing::info!("Cleaned up {} stopped services", removed);
        }
        removed
    }

    /// Get the number of registered services
    ///
    /// 获取已注册的服务数量
    pub async fn service_count(&self) -> usize {
        self.services.lock().await.len()
    }

    /// Check if any services are currently running
    ///
    /// 检查是否有任何服务正在运行
    pub async fn has_running_services(&self) -> bool {
        let services = self.services.lock().await;
        services.values().any(|h| h.is_running())
    }
}

impl Default for BackgroundServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BackgroundServiceManager {
    fn drop(&mut self) {
        // If manager is dropped, try to send shutdown signal (non-blocking)
        //
        // 如果管理器被丢弃，尝试发送关闭信号（不阻塞）
        let _ = self.shutdown_sender.send(());
    }
}
