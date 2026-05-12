use std::collections::HashMap;
use sysinfo::{Disks, Networks, Pid, Process, ProcessesToUpdate, System};

/// `SystemInfo` is a wrapper around `sysinfo::System` that provides
/// convenient methods to retrieve system information such as CPU usage,
/// memory statistics, processes, and system metadata.
///
/// # Examples
///
/// ```
/// let mut sys_info = SystemInfo::new();
/// let cpu_usage = sys_info.get_cpu_usage();
/// println!("Current CPU usage: {:.2}%", cpu_usage);
/// ```
pub struct SystemInfo(System);

impl SystemInfo {
    /// Creates a new `SystemInfo` instance with all system information
    /// initialized, including CPU, memory, and process data.
    ///
    /// This method also performs an initial CPU refresh to ensure
    /// accurate measurements when `get_cpu_usage()` is called.
    ///
    /// # Returns
    ///
    /// A new `SystemInfo` instance with fully refreshed system data.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// ```
    pub fn new() -> Self {
        let mut sys = System::new_all(); // Initialize all system information (CPU, memory, processes, etc.)
        sys.refresh_cpu_all();
        Self(sys)
    }

    /// Returns the global CPU usage as a percentage.
    ///
    /// This method refreshes all CPU data before reading to provide
    /// the most up-to-date measurement. The returned value is the
    /// average usage across all CPU cores.
    ///
    /// # Returns
    ///
    /// A `f32` value representing the global CPU usage percentage,
    /// ranging from `0.0` to `100.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// let usage = sys_info.get_cpu_usage();
    /// assert!(usage >= 0.0 && usage <= 100.0);
    /// ```
    pub fn get_cpu_usage(&mut self) -> f32 {
        self.0.refresh_cpu_all();
        self.0.global_cpu_usage() // Returns global CPU usage
    }

    /// Returns the number of logical CPU cores available on the system.
    ///
    /// # Returns
    ///
    /// A `usize` representing the count of logical CPU cores.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// let count = sys_info.get_cpu_count();
    /// assert!(count > 0);
    /// ```
    pub fn get_cpu_count(&mut self) -> usize {
        self.0.refresh_cpu_all();
        self.0.cpus().len() // Returns the number of logical CPU cores
    }

    /// Returns a slice containing information about all CPU cores.
    ///
    /// Each [`sysinfo::Cpu`] in the returned slice provides detailed
    /// statistics such as usage percentage, frequency, and vendor ID.
    ///
    /// # Returns
    ///
    /// A slice of [`sysinfo::Cpu`] structs, one for each logical core.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// for cpu in sys_info.get_cpus() {
    ///     println!("CPU {}: {:.2}%", cpu.name(), cpu.cpu_usage());
    /// }
    /// ```
    pub fn get_cpus(&mut self) -> &[sysinfo::Cpu] {
        self.0.refresh_cpu_all();
        self.0.cpus() // Returns information for all CPU cores
    }

    /// Returns the amount of used system memory in bytes.
    ///
    /// This includes memory actively in use by processes and the kernel,
    /// but does not include memory that is free or available for reuse.
    ///
    /// # Returns
    ///
    /// A `u64` value representing the used memory in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// let used = sys_info.get_memory_usage();
    /// let total = sys_info.get_total_memory();
    /// assert!(used <= total);
    /// ```
    pub fn get_memory_usage(&mut self) -> u64 {
        self.0.refresh_memory();
        self.0.used_memory() // Returns the amount of used memory
    }

    /// Returns the amount of available system memory in bytes.
    ///
    /// Available memory is an estimate of how much memory can be
    /// allocated without causing swapping. It typically includes
    /// free memory plus reclaimable caches and buffers.
    ///
    /// # Returns
    ///
    /// A `u64` value representing the available memory in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// let available = sys_info.get_memory_available();
    /// println!("Available memory: {} bytes", available);
    /// ```
    pub fn get_memory_available(&mut self) -> u64 {
        self.0.refresh_memory(); // Refresh memory information
        self.0.available_memory() // Returns the amount of available memory
    }

    /// Returns the total amount of physical system memory in bytes.
    ///
    /// This is the total RAM installed in the system, including memory
    /// reserved for hardware and the kernel.
    ///
    /// # Returns
    ///
    /// A `u64` value representing the total memory in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// let total = sys_info.get_total_memory();
    /// assert!(total > 0);
    /// ```
    pub fn get_total_memory(&mut self) -> u64 {
        self.0.refresh_memory();
        self.0.total_memory() // Returns the total amount of memory
    }

    /// Returns the amount of used swap space in bytes.
    ///
    /// Swap space is disk storage used as virtual memory when physical
    /// RAM is fully utilized.
    ///
    /// # Returns
    ///
    /// A `u64` value representing the used swap space in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// let used_swap = sys_info.get_swap_usage();
    /// let total_swap = sys_info.get_total_swap();
    /// assert!(used_swap <= total_swap);
    /// ```
    pub fn get_swap_usage(&mut self) -> u64 {
        self.0.refresh_memory();
        self.0.used_swap() // Returns the amount of used swap space
    }

    /// Returns the total amount of swap space in bytes.
    ///
    /// # Returns
    ///
    /// A `u64` value representing the total swap space in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// let total_swap = sys_info.get_total_swap();
    /// println!("Total swap: {} bytes", total_swap);
    /// ```
    pub fn get_total_swap(&mut self) -> u64 {
        self.0.refresh_memory();
        self.0.total_swap() // Returns the total amount of swap space
    }

    /// Returns a reference to the hash map of all currently tracked processes,
    /// keyed by their [`Pid`].
    ///
    /// # Returns
    ///
    /// A reference to a [`HashMap<Pid, Process>`] containing all processes.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// let processes = sys_info.get_process();
    /// for (pid, process) in processes {
    ///     println!("PID {}: {}", pid, process.name());
    /// }
    /// ```
    pub fn get_process(&mut self) -> &HashMap<Pid, Process> {
        self.0.processes() // Returns all processes keyed by PID
    }

    /// Returns the number of physical CPU cores, if supported by the system.
    ///
    /// This differs from [`get_cpu_count`](Self::get_cpu_count) in that it
    /// returns the count of physical cores rather than logical cores
    /// (which may include hyper-threaded cores).
    ///
    /// # Returns
    ///
    /// An `Option<usize>` containing the physical core count if available,
    /// or `None` if the information cannot be determined.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// if let Some(physical_cores) = sys_info.get_physical_core_count() {
    ///     println!("Physical cores: {}", physical_cores);
    /// }
    /// ```
    pub fn get_physical_core_count(&mut self) -> Option<usize> {
        self.0.physical_core_count() // Returns the number of physical cores
    }

    /// Returns an iterator over processes whose name matches the given string.
    ///
    /// The search is case-sensitive. Processes are refreshed before the search
    /// to ensure the most recent process list is used.
    ///
    /// # Arguments
    ///
    /// * `name` - A string slice containing the process name to search for.
    ///
    /// # Returns
    ///
    /// An iterator yielding references to [`Process`] structs whose names
    /// match the provided search string.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// for process in sys_info.get_process_from_name("firefox") {
    ///     println!("Found process: PID {}", process.pid());
    /// }
    /// ```
    pub fn get_process_from_name<'a: 'b, 'b>(
        &'a mut self,
        name: &'b str,
    ) -> impl Iterator<Item = &'a Process> + 'b {
        self.0.refresh_processes(ProcessesToUpdate::All, true); // Refresh all process information
        self.0.processes_by_name(name.as_ref()) // Returns processes matching the given name
    }

    /// Returns the process with the specified PID, if it exists.
    ///
    /// All processes are refreshed before the lookup to ensure the most
    /// recent data is available.
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID as a `u32` value.
    ///
    /// # Returns
    ///
    /// An `Option<&Process>` containing a reference to the process if found,
    /// or `None` if no process with the given PID exists.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// if let Some(process) = sys_info.get_process_from_pid(1) {
    ///     println!("Process name: {}", process.name());
    /// }
    /// ```
    pub fn get_process_from_pid(&mut self, pid: u32) -> Option<&Process> {
        self.0.refresh_processes(ProcessesToUpdate::All, true); // Refresh all process information
        self.0.process(Pid::from_u32(pid)) // Returns the process with the given PID
    }

    /// Returns the total number of currently running processes.
    ///
    /// Processes are refreshed before counting to provide an accurate
    /// count of the current process landscape.
    ///
    /// # Returns
    ///
    /// A `usize` representing the total number of processes.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sys_info = SystemInfo::new();
    /// let count = sys_info.get_process_count();
    /// assert!(count > 0);
    /// ```
    pub fn get_process_count(&mut self) -> usize {
        self.0.refresh_processes(ProcessesToUpdate::All, true); // Refresh all process information
        self.0.processes().len() // Returns the total number of processes
    }
}

impl SystemInfo {
    /// Returns the name of the operating system.
    ///
    /// This is a static method that does not require an instance of `SystemInfo`.
    /// The returned name is typically something like "Linux", "Windows", or "macOS".
    ///
    /// # Returns
    ///
    /// An `Option<String>` containing the OS name if available,
    /// or `None` if it cannot be determined.
    ///
    /// # Examples
    ///
    /// ```
    /// if let Some(name) = SystemInfo::name() {
    ///     println!("OS: {}", name);
    /// }
    /// ```
    pub fn name() -> Option<String> {
        System::name() // Returns the system name
    }

    /// Returns the version of the operating system.
    ///
    /// This is a static method that does not require an instance of `SystemInfo`.
    ///
    /// # Returns
    ///
    /// An `Option<String>` containing the OS version string if available,
    /// or `None` if it cannot be determined.
    ///
    /// # Examples
    ///
    /// ```
    /// if let Some(version) = SystemInfo::os_version() {
    ///     println!("OS Version: {}", version);
    /// }
    /// ```
    pub fn os_version() -> Option<String> {
        System::os_version() // Returns the operating system version
    }

    /// Returns the hostname of the system.
    ///
    /// This is a static method that does not require an instance of `SystemInfo`.
    /// The hostname is typically the network name assigned to the machine.
    ///
    /// # Returns
    ///
    /// An `Option<String>` containing the hostname if available,
    /// or `None` if it cannot be determined.
    ///
    /// # Examples
    ///
    /// ```
    /// if let Some(host) = SystemInfo::host_name() {
    ///     println!("Hostname: {}", host);
    /// }
    /// ```
    pub fn host_name() -> Option<String> {
        System::host_name() // Returns the hostname
    }

    /// Returns the kernel version of the operating system.
    ///
    /// This is a static method that does not require an instance of `SystemInfo`.
    /// On Linux, this returns the kernel release string (e.g., "5.15.0-91-generic").
    ///
    /// # Returns
    ///
    /// An `Option<String>` containing the kernel version if available,
    /// or `None` if it cannot be determined.
    ///
    /// # Examples
    ///
    /// ```
    /// if let Some(kernel) = SystemInfo::kernel_version() {
    ///     println!("Kernel: {}", kernel);
    /// }
    /// ```
    pub fn kernel_version() -> Option<String> {
        System::kernel_version() // Returns the kernel version
    }

    /// Returns network interface information with freshly refreshed data.
    ///
    /// This is a static method that does not require an instance of `SystemInfo`.
    /// The returned [`Networks`] struct provides information about all network
    /// interfaces, including received/transmitted bytes, packets, and errors.
    ///
    /// # Returns
    ///
    /// A [`Networks`] instance containing refreshed network data for all interfaces.
    ///
    /// # Examples
    ///
    /// ```
    /// let networks = SystemInfo::networks();
    /// for (interface_name, data) in &networks {
    ///     println!("{}: {} bytes received", interface_name, data.received());
    /// }
    /// ```
    pub fn networks() -> Networks {
        Networks::new_with_refreshed_list() // Returns network interfaces with refreshed data
    }

    /// Returns disk information with freshly refreshed data.
    ///
    /// This is a static method that does not require an instance of `SystemInfo`.
    /// The returned [`Disks`] struct provides information about all mounted disks,
    /// including available space, total space, and file system type.
    ///
    /// # Returns
    ///
    /// A [`Disks`] instance containing refreshed data for all mounted disks.
    ///
    /// # Examples
    ///
    /// ```
    /// let disks = SystemInfo::disks();
    /// for disk in &disks {
    ///     println!("{}: {} bytes available", disk.name().to_string_lossy(), disk.available_space());
    /// }
    /// ```
    pub fn disks() -> Disks {
        Disks::new_with_refreshed_list() // Returns disk information with refreshed data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info_creation() {
        let mut system_info = SystemInfo::new();
        assert!(system_info.get_cpu_count() > 0); // Ensure at least one logical CPU core is present
    }

    #[test]
    fn test_cpu_usage() {
        let mut system_info = SystemInfo::new();
        let cpu_usage = system_info.get_cpu_usage();
        assert!(cpu_usage >= 0.0 && cpu_usage <= 100.0); // CPU usage should be between 0% and 100%
    }

    #[test]
    fn test_memory_usage() {
        let mut system_info = SystemInfo::new();
        let used_memory = system_info.get_memory_usage();
        let total_memory = system_info.get_total_memory();
        assert!(used_memory <= total_memory); // Used memory should not exceed total memory
    }

    #[test]
    fn test_swap_usage() {
        let mut system_info = SystemInfo::new();
        let used_swap = system_info.get_swap_usage();
        let total_swap = system_info.get_total_swap();
        assert!(used_swap <= total_swap); // Used swap should not exceed total swap space
    }

    #[test]
    fn test_process_count() {
        let mut system_info = SystemInfo::new();
        let process_count = system_info.get_process_count();
        assert!(process_count > 0); // Ensure at least one process is running
    }

    #[test]
    fn test_physical_core_count() {
        let mut system_info = SystemInfo::new();
        if let Some(physical_cores) = system_info.get_physical_core_count() {
            assert!(physical_cores > 0); // Ensure physical core count is greater than 0
        }
    }

    #[test]
    fn test_system_name() {
        let system_name = SystemInfo::name();
        assert!(system_name.is_some()); // Ensure system name is available
    }

    #[test]
    fn test_os_version() {
        let os_version = SystemInfo::os_version();
        assert!(os_version.is_some()); // Ensure OS version is available
    }

    #[test]
    fn test_host_name() {
        let host_name = SystemInfo::host_name();
        assert!(host_name.is_some()); // Ensure hostname is available
    }

    #[test]
    fn test_kernel_version() {
        let kernel_version = SystemInfo::kernel_version();
        assert!(kernel_version.is_some()); // Ensure kernel version is available
    }
}
