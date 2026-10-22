//! Test harness: a mock upstream, a minimal HTTP/1.1 client and a gateway
//! process started from the real binary.
//!
//! The gateway is started as a subprocess (`CARGO_BIN_EXE_next-web-gateway`)
//! rather than by driving pingora's `Service` directly: that exercises the
//! binary, the CLI, the config loader and both listeners, which is what the
//! acceptance criteria are about.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// One request the mock upstream received.
#[derive(Debug, Clone)]
pub struct RecordedRequest {
    /// Request line target, path and query.
    pub target: String,
    /// Method token.
    pub method: String,
    /// Headers as received, lowercased names.
    pub headers: Vec<(String, String)>,
    /// Body bytes, decoded from `Content-Length`.
    pub body: Vec<u8>,
}

impl RecordedRequest {
    /// First header with that name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// What the mock upstream answers.
#[derive(Debug, Clone)]
pub enum Behavior {
    /// Answer with a fixed status, headers and body.
    Respond(u16, Vec<(String, String)>, Vec<u8>),
    /// Echo the request path in the body.
    Echo,
    /// Answer with chunked transfer encoding.
    Chunked(Vec<Vec<u8>>),
    /// Close the connection without answering.
    Close,
}

/// A tiny HTTP/1.1 server used as the backend under test.
pub struct MockUpstream {
    /// Address the server listens on.
    pub addr: SocketAddr,
    /// Every request the server received, in order.
    pub requests: Arc<Mutex<Vec<RecordedRequest>>>,
    shutdown: Arc<Mutex<bool>>,
}

impl MockUpstream {
    /// Start the server on an ephemeral port.
    pub fn start(behavior: Behavior) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock upstream");
        let addr = listener.local_addr().expect("mock upstream address");
        listener
            .set_nonblocking(true)
            .expect("non blocking mock upstream");

        let requests = Arc::new(Mutex::new(Vec::new()));
        let shutdown = Arc::new(Mutex::new(false));

        let thread_requests = Arc::clone(&requests);
        let thread_shutdown = Arc::clone(&shutdown);

        std::thread::spawn(move || {
            while !*thread_shutdown.lock().expect("shutdown flag") {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let requests = Arc::clone(&thread_requests);
                        let behavior = behavior.clone();
                        std::thread::spawn(move || handle(stream, behavior, requests));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            addr,
            requests,
            shutdown,
        }
    }

    /// Last request received.
    pub fn last_request(&self) -> Option<RecordedRequest> {
        self.requests.lock().ok()?.last().cloned()
    }

    /// Number of requests received.
    pub fn request_count(&self) -> usize {
        self.requests.lock().map(|r| r.len()).unwrap_or(0)
    }
}

impl Drop for MockUpstream {
    fn drop(&mut self) {
        if let Ok(mut flag) = self.shutdown.lock() {
            *flag = true;
        }
    }
}

fn handle(mut stream: TcpStream, behavior: Behavior, requests: Arc<Mutex<Vec<RecordedRequest>>>) {
    stream.set_nonblocking(false).ok();
    stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .ok();

    let Some(request) = read_request(&mut stream) else {
        return;
    };

    if let Ok(mut requests) = requests.lock() {
        requests.push(request.clone());
    }

    match behavior {
        Behavior::Close => return,
        Behavior::Respond(status, headers, body) => {
            let mut response = format!("HTTP/1.1 {status} {}\r\n", reason(status));
            for (name, value) in headers {
                response.push_str(&format!("{name}: {value}\r\n"));
            }
            response.push_str(&format!("Content-Length: {}\r\n", body.len()));
            response.push_str("Connection: close\r\n\r\n");
            stream.write_all(response.as_bytes()).ok();
            stream.write_all(&body).ok();
        }
        Behavior::Echo => {
            let body = request.target.clone().into_bytes();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(response.as_bytes()).ok();
            stream.write_all(&body).ok();
        }
        Behavior::Chunked(chunks) => {
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                )
                .ok();
            for chunk in chunks {
                stream
                    .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                    .ok();
                stream.write_all(&chunk).ok();
                stream.write_all(b"\r\n").ok();
            }
            stream.write_all(b"0\r\n\r\n").ok();
        }
    }

    stream.flush().ok();
}

fn read_request(stream: &mut TcpStream) -> Option<RecordedRequest> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];

    // Read until the header block is complete.
    loop {
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let header_end = buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")?
        + 4;
    let head = String::from_utf8_lossy(&buffer[..header_end]).into_owned();
    let mut lines = head.split("\r\n");

    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();

    let headers: Vec<(String, String)> = lines
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect();

    let content_length = headers
        .iter()
        .find(|(name, _)| name == "content-length")
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);

    let mut body = buffer[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }

    Some(RecordedRequest {
        target,
        method,
        headers,
        body,
    })
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Status",
    }
}

/// A parsed HTTP response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// Status code.
    pub status: u16,
    /// Headers as received, lowercased names.
    pub headers: Vec<(String, String)>,
    /// Decoded body.
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// First header with that name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Body interpreted as UTF-8.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Send one request to `addr` and read the response.
pub fn request(
    addr: SocketAddr,
    method: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> HttpResponse {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))
        .expect("connect to the gateway");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("read timeout");

    let caller_sets_host = headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("host"));

    let mut head = format!("{method} {target} HTTP/1.1\r\n");
    if !caller_sets_host {
        head.push_str("Host: test\r\n");
    }
    head.push_str("Connection: close\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    if let Some(body) = body {
        head.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    head.push_str("\r\n");

    stream.write_all(head.as_bytes()).expect("write request head");
    if let Some(body) = body {
        stream.write_all(body).expect("write request body");
    }
    stream.flush().ok();

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).expect("read the response");
    parse_response(&raw)
}

/// GET helper.
pub fn get(addr: SocketAddr, target: &str, headers: &[(&str, &str)]) -> HttpResponse {
    request(addr, "GET", target, headers, None)
}

/// POST a chunked body, so no `Content-Length` is advertised.
pub fn post_chunked(addr: SocketAddr, target: &str, chunks: &[&[u8]]) -> HttpResponse {
    let mut stream =
        TcpStream::connect_timeout(&addr, Duration::from_secs(2)).expect("connect to the gateway");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("read timeout");

    let head = format!(
        "POST {target} HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\
         Transfer-Encoding: chunked\r\n\r\n"
    );
    stream.write_all(head.as_bytes()).expect("write request head");

    for chunk in chunks {
        stream
            .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
            .expect("write chunk size");
        stream.write_all(chunk).expect("write chunk");
        stream.write_all(b"\r\n").expect("write chunk terminator");
    }
    stream.write_all(b"0\r\n\r\n").expect("write last chunk");
    stream.flush().ok();

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).expect("read the response");
    parse_response(&raw)
}

fn parse_response(raw: &[u8]) -> HttpResponse {
    let header_end = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
        .unwrap_or(raw.len());

    let head = String::from_utf8_lossy(&raw[..header_end]).into_owned();
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(0);

    let headers: Vec<(String, String)> = lines
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect();

    let body = &raw[header_end.min(raw.len())..];
    let chunked = headers
        .iter()
        .any(|(name, value)| name == "transfer-encoding" && value.contains("chunked"));

    HttpResponse {
        status,
        headers,
        body: if chunked {
            decode_chunked(body)
        } else {
            body.to_vec()
        },
    }
}

fn decode_chunked(mut raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    while let Some(position) = raw.windows(2).position(|window| window == b"\r\n") {
        let size_line = String::from_utf8_lossy(&raw[..position]).into_owned();
        let Ok(size) = usize::from_str_radix(size_line.trim(), 16) else {
            break;
        };
        raw = &raw[position + 2..];
        if size == 0 {
            break;
        }
        if raw.len() < size {
            out.extend_from_slice(raw);
            break;
        }
        out.extend_from_slice(&raw[..size]);
        raw = &raw[size..];
        if raw.starts_with(b"\r\n") {
            raw = &raw[2..];
        }
    }

    out
}

/// A gateway process serving a generated configuration.
pub struct TestGateway {
    /// Proxy listener address.
    pub addr: SocketAddr,
    /// Admin listener address.
    pub admin: SocketAddr,
    child: Child,
    config_path: std::path::PathBuf,
    log_path: std::path::PathBuf,
    out_path: std::path::PathBuf,
}

impl TestGateway {
    /// Start the gateway with `routes` as the `routes:` list body.
    ///
    /// A Rust `\`-continued literal loses the indentation of its first line, so
    /// a block that starts at column 0 is re-indented to sit under `routes:`.
    pub fn start(routes: &str) -> Self {
        let routes = indent_first_line(routes);

        Self::start_with(|proxy, admin| {
            format!(
                "next:
  gateway:
    server:
      bind: \"127.0.0.1:{proxy}\"
      admin:
        enabled: true
        bind: \"127.0.0.1:{admin}\"
    logging:
      access_log: true
    routes:
{routes}"
            )
        })
    }

    /// Start the gateway from a document built with the two listener ports.
    pub fn start_with(build: impl Fn(u16, u16) -> String) -> Self {
        // Ports are picked by binding and releasing them, so a busy port is a
        // real possibility when several test binaries run in parallel. Retry a
        // few times before failing.
        let mut last_log = String::new();

        for _ in 0..3 {
            match Self::try_start(&build) {
                Ok(gateway) => return gateway,
                Err(log) => last_log = log,
            }
        }

        panic!("the gateway did not start after 3 attempts; last log:\n{last_log}");
    }

    fn try_start(build: &impl Fn(u16, u16) -> String) -> Result<Self, String> {
        let proxy_port = free_port();
        let admin_port = free_port();
        let document = build(proxy_port, admin_port);

        let config_path =
            std::env::temp_dir().join(format!("next-web-gateway-{proxy_port}.yaml"));
        std::fs::write(&config_path, document.clone()).expect("write the test configuration");

        let log_path = std::env::temp_dir().join(format!("next-web-gateway-{proxy_port}.log"));
        let log = std::fs::File::create(&log_path).expect("create the gateway log");
        let out_path = std::env::temp_dir().join(format!("next-web-gateway-{proxy_port}.out"));
        let out = std::fs::File::create(&out_path).expect("create the gateway stdout");

        let mut child = Command::new(env!("CARGO_BIN_EXE_next-web-gateway"))
            .arg("--config")
            .arg(&config_path)
            .arg("--log-level")
            .arg("info")
            .stdout(Stdio::from(out))
            .stderr(Stdio::from(log))
            .spawn()
            .expect("start the gateway binary");

        let addr: SocketAddr = format!("127.0.0.1:{proxy_port}").parse().expect("proxy address");
        let admin: SocketAddr = format!("127.0.0.1:{admin_port}").parse().expect("admin address");

        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if TcpStream::connect_timeout(&admin, Duration::from_millis(50)).is_ok() {
                return Ok(Self {
                    addr,
                    admin,
                    child,
                    config_path,
                    log_path,
                    out_path,
                });
            }

            if let Ok(Some(status)) = child.try_wait() {
                let log = std::fs::read_to_string(&log_path).unwrap_or_default();
                let _ = std::fs::remove_file(&config_path);
                let _ = std::fs::remove_file(&log_path);
                let _ = std::fs::remove_file(&out_path);
                return Err(format!("exited with {status}, config:\n{document}\nlog:\n{log}"));
            }

            std::thread::sleep(Duration::from_millis(25));
        }

        let _ = child.kill();
        let _ = child.wait();
        let log = std::fs::read_to_string(&log_path).unwrap_or_default();
        let _ = std::fs::remove_file(&config_path);
        let _ = std::fs::remove_file(&log_path);
        let _ = std::fs::remove_file(&out_path);

        Err(format!("timed out, config:\n{document}\nlog:\n{log}"))
    }

    /// Everything the gateway wrote to stdout so far.
    pub fn stdout(&self) -> String {
        std::fs::read_to_string(&self.out_path).unwrap_or_default()
    }
}

impl Drop for TestGateway {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.config_path);
        let _ = std::fs::remove_file(&self.log_path);
    }
}

/// Bind an ephemeral port and immediately release it.
pub fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind an ephemeral port");
    let port = listener.local_addr().expect("ephemeral address").port();
    drop(listener);
    port
}

/// Re-indent a YAML block so its list items sit under a `routes:` key at four
/// spaces. A `\`-continued literal loses the indentation of its first line, so
/// the block is normalised by its common leading whitespace first.
fn indent_first_line(block: &str) -> String {
    let lines: Vec<&str> = block
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    if lines.is_empty() {
        return String::new();
    }

    let common = lines
        .iter()
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    lines
        .iter()
        .map(|line| format!("      {}", &line[common.min(line.len())..]))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Run `body` against a mock upstream and a gateway route pointing at it.
pub fn with_upstream(behavior: Behavior, template: &str) -> (MockUpstream, TestGateway) {
    let upstream = MockUpstream::start(behavior);
    let routes = template.replace("{upstream}", &upstream.addr.to_string());
    let gateway = TestGateway::start(&routes);

    (upstream, gateway)
}
