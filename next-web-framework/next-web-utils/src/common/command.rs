use std::{
    io::{BufRead, BufReader, Error, ErrorKind},
    process::{Command, Stdio},
};

use std::sync::mpsc::Sender;
pub struct CommandUtil;

/// 运行命令工具类
impl CommandUtil {
    //! 运行命令 例如 linux shell 命令
    pub fn exec(program: &str, args: Vec<&str>) -> Result<String, Error> {
        // run the ls command

        let output = Command::new(program).args(args).output()?;

        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Command failed",
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    pub fn exec_from_str(command: &str) -> Result<String, Error> {
        let mut args = command.split_whitespace();
        let program = args.next().unwrap_or("");
        let args: Vec<&str> = args.collect::<Vec<&str>>();
        return Self::exec(program, args);
    }

    pub fn exec_and_fallback<F>(command: &str, fallback: F) -> Result<String, Error>
    where
        F: FnOnce(),
    {
        match Self::exec_from_str(command) {
            Ok(s) => Ok(s),
            Err(e) => {
                fallback();
                Err(e)
            }
        }
    }

    /// 执行命令并将持续输出通过通道发送。
    ///
    /// # 参数
    /// - `command`: 要执行的命令字符串。
    /// - `sender`: 一个 `Sender<String>`，用于发送命令的输出。
    /// - `block`: 是否阻塞当前线程以等待命令完成。
    ///
    /// # 返回值
    /// 如果命令执行成功，则返回 `Ok(())`；否则返回错误 <button class="citation-flag" data-index="1">。
    pub fn exec_to_channel(
        command: &str,
        sender: Sender<String>,
        block: bool,
    ) -> Result<(), Error> {
        // 分割命令字符串为程序名和参数
        let mut args = command.split_whitespace();
        let program = args.next().unwrap_or("");
        let args: Vec<&str> = args.collect();

        // 启动子进程并捕获标准输出
        let mut child = Command::new(program)
            .args(args)
            .stdout(Stdio::piped()) // 捕获标准输出
            .spawn()?;

        // 获取子进程的标准输出流
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::new(ErrorKind::Other, "无法获取子进程的标准输出"))?;

        // 使用缓冲区逐行读取子进程的输出
        let mut reader = BufReader::new(stdout);
        

        let mut func = move || -> Result<(), Error> {
            loop {
                let mut buffer = String::new();
                let bytes_read = reader.read_line(&mut buffer)?; // 读取一行输出
                if bytes_read == 0 {
                    break; // 子进程输出结束
                }

                // 将读取到的内容通过通道发送
                if sender.send(buffer.clone()).is_err() {
                    break; // 如果发送失败，退出循环
                }
            }

            // 等待子进程结束
            let _ = child.wait()?;
            Ok(())
        };

        if block {
            func() // 阻塞模式下直接运行
        } else {
            std::thread::spawn(func); // 非阻塞模式下在新线程中运行
            Ok(())
        }
    }
}
