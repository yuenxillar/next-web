use std::collections::HashMap;
use sysinfo::{Disks, Networks, Pid, Process, ProcessesToUpdate, System};

/// `SystemInfo` 是一个封装了 `sysinfo::System` 的结构体，用于获取系统信息。
pub struct SystemInfo(System);

impl SystemInfo {
    /// 创建一个新的 `SystemInfo` 实例，并初始化所有系统信息。
    pub fn new() -> Self {
        let sys = System::new_all(); // 初始化所有系统信息（CPU、内存、进程等)
        Self(sys)
    }

    /// 获取全局 CPU 使用率（百分比）。
    pub fn get_cpu_usage(&mut self) -> f32 {
        self.0.refresh_cpu_all(); // 刷新所有 CPU 信息
        self.0.global_cpu_usage() // 返回全局 CPU 使用率
    }

    /// 获取逻辑 CPU 核心数量。
    pub fn get_cpu_count(&mut self) -> usize {
        self.0.refresh_cpu_all(); // 刷新所有 CPU 信息
        self.0.cpus().len() // 返回逻辑 CPU 核心数量
    }

    /// 获取所有 CPU 核心的信息。
    pub fn get_cpus(&mut self) -> &[sysinfo::Cpu] {
        self.0.refresh_cpu_all(); // 刷新所有 CPU 信息
        self.0.cpus() // 返回所有 CPU 核心的信息
    }

    /// 获取已使用的内存大小（单位：字节）。
    pub fn get_memory_usage(&mut self) -> u64 {
        self.0.refresh_memory(); // 刷新内存信息
        self.0.used_memory() // 返回已使用的内存大小
    }

    /// 获取可用内存大小（单位：字节）。
    pub fn get_memory_available(&mut self) -> u64 {
        self.0.refresh_memory(); // 刷新内存信息
        self.0.available_memory() // 返回可用内存大小
    }

    /// 获取总内存大小（单位：字节）。
    pub fn get_total_memory(&mut self) -> u64 {
        self.0.refresh_memory(); // 刷新内存信息
        self.0.total_memory() // 返回总内存大小
    }

    /// 获取已使用的交换空间大小（单位：字节）。
    pub fn get_swap_usage(&mut self) -> u64 {
        self.0.refresh_memory(); // 刷新内存信息
        self.0.used_swap() // 返回已使用的交换空间大小
    }

    /// 获取总交换空间大小（单位：字节）。
    pub fn get_total_swap(&mut self) -> u64 {
        self.0.refresh_memory(); // 刷新内存信息
        self.0.total_swap() // 返回总交换空间大小
    }

    /// 获取当前所有进程的哈希表（PID -> 进程信息）。
    pub fn get_process(&mut self) -> &HashMap<Pid, Process> {
        self.0.processes() // 返回所有进程的哈希表
    }

    /// 获取物理核心数量（如果支持）。
    pub fn get_physical_core_count(&mut self) -> Option<usize> {
        self.0.physical_core_count() // 返回物理核心数量
    }

    /// 根据进程名称获取匹配的进程列表。
    pub fn get_process_from_name<'a: 'b, 'b>(
        &'a mut self,
        name: &'b str,
    ) -> impl Iterator<Item = &'a Process> + 'b {
        self.0.refresh_processes(ProcessesToUpdate::All, true); // 刷新所有进程信息
        self.0.processes_by_name(name.as_ref()) // 返回匹配的进程迭代器
    }

    /// 根据 PID 获取指定的进程信息。
    pub fn get_process_from_pid(&mut self, pid: u32) -> Option<&Process> {
        self.0.refresh_processes(ProcessesToUpdate::All, true); // 刷新所有进程信息
        self.0.process(Pid::from_u32(pid)) // 返回指定 PID 的进程信息
    }

    /// 获取当前运行的进程数量。
    pub fn get_process_count(&mut self) -> usize {
        self.0.refresh_processes(ProcessesToUpdate::All, true); // 刷新所有进程信息
        self.0.processes().len() // 返回进程数量
    }
}

impl SystemInfo {
    /// 获取系统的名称（如操作系统名称）。
    pub fn name() -> Option<String> {
        System::name() // 返回系统名称
    }

    /// 获取操作系统的版本号。
    pub fn os_version() -> Option<String> {
        System::os_version() // 返回操作系统版本号
    }

    /// 获取主机名。
    pub fn host_name() -> Option<String> {
        System::host_name() // 返回主机名
    }

    /// 获取内核版本号。
    pub fn kernel_version() -> Option<String> {
        System::kernel_version() // 返回内核版本号
    }

    /// 获取网络接口信息。
    pub fn networks() -> Networks {
        Networks::new_with_refreshed_list() // 返回刷新后的网络接口信息
    }

    /// 获取磁盘信息。
    pub fn disks() -> Disks {
        Disks::new_with_refreshed_list() // 返回刷新后的磁盘信息
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info_creation() {
        let mut system_info = SystemInfo::new();
        assert!(system_info.get_cpu_count() > 0); // 确保至少有一个逻辑 CPU 核心
    }

    #[test]
    fn test_cpu_usage() {
        let mut system_info = SystemInfo::new();
        let cpu_usage = system_info.get_cpu_usage();
        assert!(cpu_usage >= 0.0 && cpu_usage <= 100.0); // CPU 使用率应在 0% 到 100% 之间
    }

    #[test]
    fn test_memory_usage() {
        let mut system_info = SystemInfo::new();
        let used_memory = system_info.get_memory_usage();
        let total_memory = system_info.get_total_memory();
        assert!(used_memory <= total_memory); // 已使用内存不应超过总内存
    }

    #[test]
    fn test_swap_usage() {
        let mut system_info = SystemInfo::new();
        let used_swap = system_info.get_swap_usage();
        let total_swap = system_info.get_total_swap();
        assert!(used_swap <= total_swap); // 已使用交换空间不应超过总交换空间
    }

    #[test]
    fn test_process_count() {
        let mut system_info = SystemInfo::new();
        let process_count = system_info.get_process_count();
        assert!(process_count > 0); // 确保至少有一个进程在运行
    }

    #[test]
    fn test_physical_core_count() {
        let mut system_info = SystemInfo::new();
        if let Some(physical_cores) = system_info.get_physical_core_count() {
            assert!(physical_cores > 0); // 确保物理核心数量大于 0
        }
    }

    #[test]
    fn test_system_name() {
        let system_name = SystemInfo::name();
        assert!(system_name.is_some()); // 确保系统名称存在
    }

    #[test]
    fn test_os_version() {
        let os_version = SystemInfo::os_version();
        assert!(os_version.is_some()); // 确保操作系统版本号存在
    }

    #[test]
    fn test_host_name() {
        let host_name = SystemInfo::host_name();
        assert!(host_name.is_some()); // 确保主机名存在
    }

    #[test]
    fn test_kernel_version() {
        let kernel_version = SystemInfo::kernel_version();
        assert!(kernel_version.is_some()); // 确保内核版本号存在
    }
}