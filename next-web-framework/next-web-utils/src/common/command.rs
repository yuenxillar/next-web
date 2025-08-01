use std::io::{BufRead, BufReader, Error, ErrorKind};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::vec::Vec;
use std::sync::mpsc::Sender;

/// 一个用于执行系统命令并处理其输出的工具结构体。
/// 
/// 此结构体目前仅作为包含静态方法的命名空间使用，不持有任何状态。
/// 它提供了多种执行命令的方式，包括获取完整输出、从字符串执行、提供备选方案以及流式输出。
/// 
/// # English
/// 
/// A utility struct for executing system commands and handling their output.
/// 
/// This struct currently serves only as a namespace for static methods and does not hold any state.
/// It provides multiple ways to execute commands, including capturing full output, executing from a string,
/// providing fallback options, and streaming output.
#[derive(Debug)]
pub struct CommandRunner;

impl CommandRunner {
    /// 执行指定的系统命令并返回其标准输出。
    /// 
    /// # 参数 (Parameters)
    /// 
    /// * `program` - 要执行的程序名称或路径（例如 `"ls"`, `"/usr/bin/git"`）。
    /// * `args` - 传递给程序的参数切片（例如 `&["-l", "/home"]`）。
    /// 
    /// # 返回值 (Returns)
    /// 
    /// 如果命令成功执行（退出状态为0），则返回包含命令标准输出的 `Result<String, Error>`。
    /// 输出会被转换为 UTF-8 字符串，无法转换的字节会被替换为 `U+FFFD`。
    /// 
    /// # 错误 (Errors)
    /// 
    /// 如果命令执行失败（例如程序不存在、权限不足）或命令本身返回非零退出状态，则返回 `Error`。
    /// 
    /// # 示例 (Example)
    /// 
    /// ```no_run
    /// let output = CommandRunner::run("echo", &["Hello, World!"]);
    /// match output {
    ///     Ok(text) => println!("{}", text),
    ///     Err(e) => eprintln!("Command failed: {}", e),
    /// }
    /// ```
    /// 
    /// # English
    /// 
    /// Executes a system command and returns its stdout output.
    /// 
    /// # Parameters
    /// 
    /// * `program` - The name or path of the program to execute (e.g., `"ls"`, `"/usr/bin/git"`).
    /// * `args` - A slice of arguments to pass to the program (e.g., `&["-l", "/home"]`).
    /// 
    /// # Returns
    /// 
    /// Returns a `Result<String, Error>` containing the command's stdout output if successful (exit status 0).
    /// The output is converted to a UTF-8 string, with invalid bytes replaced by `U+FFFD`.
    /// 
    /// # Errors
    /// 
    /// Returns an `Error` if the command fails to execute (e.g., program not found, permission denied)
    /// or if the command itself exits with a non-zero status.
    /// 
    /// # Example
    /// 
    /// ```no_run
    /// let output = CommandRunner::run("echo", &["Hello, World!"]);
    /// match output {
    ///     Ok(text) => println!("{}", text),
    ///     Err(e) => eprintln!("Command failed: {}", e),
    /// }
    /// ```
    pub fn run(program: &str, args: &[&str]) -> Result<String, Error> {
        let output = Command::new(program).args(args).output()?;
        Self::check_exit_status(output.status)?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// 从格式化的字符串中解析并执行命令。
    /// 
    /// # 参数 (Parameters)
    /// 
    /// * `command` - 包含程序名和参数的完整命令字符串，以空格分隔（例如 `"git status -s"`）。
    /// 
    /// # 返回值 (Returns)
    /// 
    /// 返回 `Result<String, Error>`，行为与 `run` 方法相同。
    /// 
    /// # 错误 (Errors)
    /// 
    /// 如果输入字符串为空或仅包含空白字符，则返回 `InvalidInput` 错误。
    /// 
    /// # 示例 (Example)
    /// 
    /// ```no_run
    /// let output = CommandRunner::run_from_str("ls -la /tmp");
    /// ```
    /// 
    /// # English
    /// 
    /// Executes a command parsed from a formatted string.
    /// 
    /// # Parameters
    /// 
    /// * `command` - A complete command string containing the program name and arguments, separated by whitespace
    ///               (e.g., `"git status -s"`).
    /// 
    /// # Returns
    /// 
    /// Returns a `Result<String, Error>` with the same behavior as the `run` method.
    /// 
    /// # Errors
    /// 
    /// Returns an `InvalidInput` error if the input string is empty or contains only whitespace.
    /// 
    /// # Example
    /// 
    /// ```no_run
    /// let output = CommandRunner::run_from_str("ls -la /tmp");
    /// ```
    pub fn run_from_str(command: &str) -> Result<String, Error> {
        let mut parts = command.split_whitespace();
        let program = parts
            .next()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Empty command string"))?;

        Self::run(program, &parts.collect::<Vec<_>>())
    }

    /// 执行命令，如果失败则调用备选函数。
    /// 
    /// # 参数 (Parameters)
    /// 
    /// * `command` - 要尝试执行的命令字符串。
    /// * `fallback` - 一个 `FnOnce` 闭包，当主命令执行失败时会被调用。
    /// 
    /// # 返回值 (Returns)
    /// 
    /// 返回主命令成功执行的结果，或者在主命令失败时返回备选函数的执行结果。
    /// 
    /// # 示例 (Example)
    /// 
    /// ```no_run
    /// let result = CommandRunner::run_with_fallback(
    ///     "nonexistent-command",
    ///     |err| println!("Fallback"); )
    /// );
    /// ```
    /// 
    /// # English
    /// 
    /// Executes a command and invokes a fallback function if it fails.
    /// 
    /// # Parameters
    /// 
    /// * `command` - The command string to attempt to execute.
    /// * `fallback` - An `FnOnce` closure that is called if the primary command fails.
    /// 
    /// # Returns
    /// 
    /// Returns the result of the primary command if successful, or the result of the fallback function if the primary fails.
    /// 
    /// # Example
    /// 
    /// ```no_run
    /// let result = CommandRunner::run_with_fallback(
    ///     "nonexistent-command",
    ///     |err| println!("Fallback"); )
    /// );
    /// ```
    pub fn run_with_fallback<F>(command: &str, fallback: F) -> Result<String, Error>
    where
        F: FnOnce(&Error),
    {
        Self::run_from_str(command).or_else(|e| { fallback(&e); Err(e) })
    }

    /// 执行命令并将输出通过异步通道实时流式传输。
    /// 
    /// # 参数 (Parameters)
    /// 
    /// * `command` - 要执行的命令字符串。
    /// * `sender` - 一个 `std::sync::mpsc::Sender<String>`，用于将命令的每一行输出发送出去。
    /// * `block` - 如果为 `true`，则此方法会阻塞当前线程直到命令执行完成。
    ///             如果为 `false`，则命令在新线程中异步执行，此方法立即返回 `Ok(())`。
    /// 
    /// # 返回值 (Returns)
    /// 
    /// * 如果 `block` 为 `true`：命令成功启动并完成时返回 `Ok(())`，失败时返回 `Error`。
    /// * 如果 `block` 为 `false`：只要命令成功启动，就返回 `Ok(())`，后续执行的错误无法在此处捕获。
    /// 
    /// # 错误 (Errors)
    /// 
    /// 返回错误的情况包括：命令字符串为空、无法捕获子进程的标准输出、读取输出时发生 I/O 错误，或命令以非零状态退出（仅在 `block=true` 时检查）。
    /// 
    /// # 注意 (Note)
    /// 
    /// 当 `block` 为 `false` 时，此方法在后台线程中运行。如果 `sender` 被接收方丢弃，读取循环会自动停止。
    /// 
    /// # 示例 (Example)
    /// 
    /// ```no_run
    /// use std::sync::mpsc;
    /// 
    /// fn main() {
    ///     let (sender, mut receiver) = mpsc::channel(100);
    ///     // 异步执行命令，不阻塞
    ///     CommandRunner::run_with_channel("ping 127.0.0.1", sender, false).unwrap();
    /// 
    ///     while let Ok(line) = receiver.recv() {
    ///         println!("Received: {}", line);
    ///     }
    /// }
    /// ```
    /// 
    /// # English
    /// 
    /// Executes a command and streams its output line-by-line through an asynchronous channel.
    /// 
    /// # Parameters
    /// 
    /// * `command` - The command string to execute.
    /// * `sender` - A `std::sync::mpsc::Sender<String>` to send each line of the command's output.
    /// * `block` - If `true`, the method blocks the current thread until the command finishes.
    ///             If `false`, the command runs asynchronously in a new thread, and the method returns immediately with `Ok(())`.
    /// 
    /// # Returns
    /// 
    /// * If `block` is `true`: Returns `Ok(())` if the command starts and completes successfully, `Error` otherwise.
    /// * If `block` is `false`: Returns `Ok(())` if the command starts successfully; errors during later execution are not captured here.
    /// 
    /// # Errors
    /// 
    /// Errors include: empty command string, failure to capture the child process's stdout, I/O errors during reading,
    /// or the command exiting with a non-zero status (only checked if `block=true`).
    /// 
    /// # Note
    /// 
    /// When `block` is `false`, this method runs in a background thread. If the `sender` is dropped by the receiver, the reading loop stops automatically.
    /// 
    /// # Example
    /// 
    /// ```no_run
    /// use std::sync::mpsc;
    /// 
    /// fn main() {
    ///     let (sender, mut receiver) = mpsc::channel(100);
    ///     // Execute command asynchronously, non-blocking
    ///     CommandRunner::run_with_channel("ping 127.0.0.1", sender, false).unwrap();
    /// 
    ///     while let Ok(line) = receiver.recv() {
    ///         println!("Received: {}", line);
    ///     }
    /// }
    /// ```
    pub fn run_with_channel(
        command: &str,
        sender: Sender<String>,
        block: bool,
    ) -> Result<(), Error> {
        let mut parts = command.split_whitespace();
        let program = parts
            .next()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Empty command string"))?;

        let mut child = Command::new(program)
            .args(parts)
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().ok_or_else(|| {
            Error::new(ErrorKind::Other, "Failed to capture child process stdout")
        })?;

        let process_output = move || -> Result<(), Error> {
            let mut reader = BufReader::new(stdout);
            let mut buffer = String::new();

            while reader.read_line(&mut buffer)? > 0 {
                if sender.send(buffer.clone()).is_err() {
                    break; // Receiver has been dropped
                }
                buffer.clear();
            }

            Self::check_exit_status(child.wait()?)?;
            Ok(())
        };

        if block {
            process_output()
        } else {
            thread::spawn(process_output);
            Ok(())
        }
    }

    /// 辅助函数：检查命令的退出状态。
    /// 
    /// # 参数 (Parameters)
    /// 
    /// * `status` - 子进程的 `ExitStatus`。
    /// 
    /// # 返回值 (Returns)
    /// 
    /// 如果状态表示成功（通常是退出码 0），则返回 `Ok(())`。
    /// 否则，返回一个包含状态信息的 `Error`。
    /// 
    /// # English
    /// 
    /// Helper function to check the exit status of a command.
    /// 
    /// # Parameters
    /// 
    /// * `status` - The `ExitStatus` of the child process.
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` if the status indicates success (typically exit code 0).
    /// Otherwise, returns an `Error` containing the status information.
    fn check_exit_status(status: ExitStatus) -> Result<(), Error> {
        if status.success() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Other,
                format!("Command failed with status: {}", status),
            ))
        }
    }
}