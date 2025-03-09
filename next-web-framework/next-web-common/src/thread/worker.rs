use std::thread;

/// 工作线程结构体，表示线程池中的一个工作线程。
pub(crate) struct Worker {
    // 工作线程的唯一标识符。
    id: usize,
    // 线程句柄，用于管理线程的生命周期。
    thread: Option<thread::JoinHandle<()>>,
    // 是否为核心线程。
    is_core: bool,
}

impl Worker {
    pub fn new(id: usize, thread: Option<thread::JoinHandle<()>>, is_core: bool) -> Self {
        Self {
            id,
            thread,
            is_core,
        }
    }

    /// 获取工作线程的 ID。
    pub fn id(&self) -> usize {
        self.id
    }

    /// 判断是否为核心线程。
    pub fn is_core(&self) -> bool {
        self.is_core
    }
}
