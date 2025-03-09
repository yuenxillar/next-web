use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;
use std::{collections::VecDeque, thread, time::Duration};

use log::{debug, error};

use super::worker::Worker;
use crate::config::thread_pool_config::ThreadPoolConfig;

// 定义任务类型，表示一个可以执行的任务。
pub(crate) type Task = Box<dyn FnOnce() + Send + 'static>;

/// 线程池结构体，用于管理线程和任务队列。
pub struct ThreadPool {
    // 工作线程列表，受互斥锁保护。
    workers: Arc<Mutex<Vec<Worker>>>,
    // 线程池配置，包含核心线程数、最大线程数等参数。
    config: ThreadPoolConfig,
    // 任务队列，存储待处理的任务。
    task_queue: Arc<Mutex<VecDeque<Task>>>,
    // 条件变量，用于线程间的同步。
    condvar: Arc<Condvar>,
    // 拒绝策略处理器，定义如何处理被拒绝的任务。
    rejected_execution_handler: Box<dyn Fn(Task, &mut VecDeque<Task>) + Send + Sync>,
    // 当前线程数，使用原子操作进行线程安全的计数。
    current_threads: Arc<AtomicUsize>,
}

impl ThreadPool {
    /// 创建一个新的线程池实例。
    ///
    /// # 参数
    /// - `config`: 线程池的配置参数。
    /// - `thread_name`: 可选的线程名称前缀。
    /// - `rejected_execution_handler`: 拒绝策略处理器。
    ///
    /// # 返回值
    /// 返回一个初始化的线程池实例。
    pub fn new(
        config: ThreadPoolConfig,
        thread_name: Option<String>,
        rejected_execution_handler: Box<dyn Fn(Task, &mut VecDeque<Task>) + Send + Sync>,
    ) -> ThreadPool {
        let task_queue = Arc::new(Mutex::new(VecDeque::with_capacity(config.get_queue_size())));
        let condvar = Arc::new(Condvar::new());
        let workers = Arc::new(Mutex::new(Vec::new()));
        let current_threads = Arc::new(AtomicUsize::new(0));

        let pool = ThreadPool {
            workers: Arc::clone(&workers),
            config: config.clone(),
            task_queue: Arc::clone(&task_queue),
            condvar: Arc::clone(&condvar),
            rejected_execution_handler,
            current_threads: Arc::clone(&current_threads),
        };

        // 初始化核心线程
        for id in 0..config.get_core_threads() {
            pool.spawn_worker(id, thread_name.clone(), true);
        }

        pool
    }

    /// 创建并启动一个新的工作线程。
    ///
    /// # 参数
    /// - `id`: 工作线程的唯一标识符。
    /// - `thread_name`: 可选的线程名称前缀。
    /// - `is_core`: 是否为核心线程。
    fn spawn_worker(&self, id: usize, thread_name: Option<String>, is_core: bool) {
        let task_queue = Arc::clone(&self.task_queue);
        let condvar = Arc::clone(&self.condvar);
        let keep_alive = if is_core {
            None // 核心线程不会因空闲而退出
        } else {
            self.config.get_keep_alive()
        };
        let current_threads = Arc::clone(&self.current_threads);

        // 定义工作线程的主体逻辑
        let body = Box::new(move || {
            debug!("Worker {} started", id);
            let mut last_active = Instant::now();

            loop {
                let task = {
                    let mut task_queue = task_queue.lock().unwrap();
                    loop {
                        if let Some(task) = task_queue.pop_front() {
                            break Some(task); // 获取任务后立即释放锁
                        }

                        // 等待任务，设置超时时间
                        let (new_queue, timeout_result) = condvar
                            .wait_timeout(task_queue, Duration::from_millis(1000))
                            .unwrap();

                        task_queue = new_queue;

                        if timeout_result.timed_out() && task_queue.is_empty() {
                            break None; // 超时且队列为空，退出线程
                        }
                    }
                };

                if let Some(task) = task {
                    debug!("Worker {} executing task", id);
                    let start_time = Instant::now();
                    task(); // 执行任务
                    debug!("Worker {} finished task in {:?}", id, start_time.elapsed());
                    last_active = Instant::now();
                }

                if let Some(keep_alive) = keep_alive {
                    if last_active.elapsed() > keep_alive {
                        debug!("Worker {} exiting due to inactivity", id);
                        current_threads.fetch_sub(1, Ordering::Relaxed);
                        break;
                    }
                }
            }
        });

        // 启动线程，并为其命名
        let handle = thread::Builder::new()
            .name(if let Some(name) = thread_name {
                format!("{}-worker-{}", name, id)
            } else {
                format!("next-web-task-worker-{}", id)
            })
            .spawn(body)
            .unwrap();

        // 将新创建的工作线程加入线程池
        if let Ok(mut workers) = self.workers.lock() {
            workers.push(Worker::new(id, Some(handle), is_core));
            self.current_threads.fetch_add(1, Ordering::Relaxed);
        } else {
            error!("Worker {} failed to spawn", id);
        }
    }

    /// 提交任务到线程池。
    ///
    /// # 参数
    /// - `f`: 需要执行的任务。
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let task = Box::new(f);
        let mut queue = match self.task_queue.lock() {
            Ok(q) => q,
            Err(_) => {
                error!("Task queue mutex poisoned");
                return;
            }
        };

        // 如果任务队列未满，则将任务加入队列
        if queue.len() < self.config.get_queue_size() {
            queue.push_back(task);
            self.condvar.notify_one();
        } else {
            let current = self.current_threads.load(Ordering::Relaxed);
            // 如果当前线程数小于最大线程数，则动态扩展线程池
            if current < self.config.get_max_threads() {
                debug!("Spawning temporary worker");
                self.spawn_worker(current + 1, Some("temporary-worker".into()), false);
                queue.push_back(task);
                self.condvar.notify_one();
            } else {
                // 使用拒绝策略处理被拒绝的任务
                (self.rejected_execution_handler)(task, &mut queue);
            }
        }
    }
}

/// 拒绝策略枚举，定义如何处理被拒绝的任务。
#[derive(Clone)]
pub enum RejectedExecutionHandler {
    // 中止策略：直接抛出异常。
    Abort,
    // 调用者运行策略：由提交任务的线程直接执行任务。
    CallerRuns,
    // 丢弃最旧任务策略：丢弃任务队列中最旧的任务，为新任务腾出空间。
    DiscardOldest,
}

impl RejectedExecutionHandler {
    /// 将拒绝策略转换为具体的处理器函数。
    pub fn to_handler(&self) -> Box<dyn Fn(Task, &mut VecDeque<Task>) + Send + Sync> {
        match self {
            Self::Abort => Box::new(|_task, _| error!("Task rejected!! no data")),
            Self::CallerRuns => Box::new(|task, _| task()),
            Self::DiscardOldest => Box::new(|task, queue| {
                queue.pop_front();
                queue.push_back(task);
            }),
        }
    }
}
