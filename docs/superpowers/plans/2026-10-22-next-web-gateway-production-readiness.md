# next-web-gateway 生产可用（Phase 1）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 `next-web-gateway` 从"能编译但不可启动的原型"变成"能上线、语义正确、可观测、有回归网"的 Spring Cloud Gateway 对应物。

**Architecture:** 自底向上重排：`config`（serde + `deny_unknown_fields` + 收集式校验，错误带 YAML 路径）→ `route`（谓词/过滤器 DSL，未知名称即启动失败）→ `upstream`（异步 DNS 缓存 + `pingora::lb::LoadBalancer`）→ `proxy`（重写 `ProxyHttp` 的三个钩子 + `logging`）→ `metrics` / `admin`（独立监听器 + Prometheus 文本）→ `Gateway` 门面 + `next-web-gateway` 二进制。旧的 `properties` / `service::jingyue_service` / `GatewayApplication` 被替换删除。

**Tech Stack:** Rust edition 2024（`rust-version = 1.98.1`）、pingora 0.9（`proxy` + `lb`，openssl 后端）、tokio、serde_yaml、serde_path_to_error、clap 4、uuid v4、pingora-limits、ipnetwork、regex。

**Spec:** `docs/superpowers/specs/2026-10-22-next-web-gateway-production-readiness-design.md`（每个任务都引用其中的决策号 D1–D14）

## Global Constraints

- `unsafe_code = "forbid"`、`missing_docs = "warn"`、`unreachable_pub = "warn"`、`rust_2018_idioms`（workspace lint，已生效）。
- 生产代码禁止 `unwrap` / `expect` / `panic!` / `unreachable!` / `println!`；日志一律 `tracing`。
- 所有 `pub` 项必须有英文文档注释。
- 构建统一：`$env:CARGO_TARGET_DIR='D:\DevProject\Rust\next-web\.cargo-target'`，`$env:TEMP='D:\DevProject\Rust\next-web\.cargo-target\tmp'`，`$env:TMP` 同上。
- 每个任务收尾三条命令全绿：`cargo test -p next-web-gateway`、`cargo clippy -p next-web-gateway --all-targets -- -D warnings`、`cargo test -p next-web-gateway --doc`。
- 提交信息 `type(scope): subject`，英文正文，一任务一提交。
- 不新增 workspace 之外的依赖；只用 spec §5 列出的既有依赖。
- 时长配置一律 `500ms` / `30s` / `2m` / `1h`；裸整数按毫秒并 `warn!`。

## Review Focus

1. **拼错的 filter / predicate 名** —— 期望启动失败并列出可用名称；绝不能静默变成 no-op（现状就是 no-op）。
2. **`https://` 上游的证书校验** —— 期望合法证书通过、自签名或域名不匹配被拒；只有配置了 `insecure_skip_verify: true` 才放行，且要 `warn!`。
3. **`lb://` 且全部上游不健康** —— 期望 503 + `Retry-After`；不能 400、不能 panic、不能挂住。
4. **chunked 请求体超过 `RequestSize`** —— 期望 413 并在超限处中断；不能因为"没有 Content-Length"就放行。
5. **访问日志与指标的一次性** —— 期望每个请求恰好一条 access log、`gateway_requests_total` 恰好加一；请求/响应两趟重复计数会让容量规划全错（现状的限流器就有这个 bug）。

---

### Task 1: `config` 模块 —— schema、时长解析、收集式校验

**Files:**
- Create: `next-web-gateway/src/config/mod.rs`、`duration.rs`、`server.rs`、`upstream.rs`、`logging.rs`、`error_model.rs`、`route.rs`、`validate.rs`
- Delete: `next-web-gateway/src/properties/{mod,gateway_properties,routes_properties,circuit_breaker_properties}.rs`
- Modify: `next-web-gateway/src/lib.rs`（`pub mod config;`）、`next-web-gateway/Cargo.toml`（加 `serde_path_to_error`）

**Interfaces:**
- `pub struct GatewayConfig { pub server: ServerConfig, pub logging: LoggingConfig, pub upstream: UpstreamConfig, pub error: ErrorConfig, pub routes: Vec<RouteConfig> }` — 全部 `#[derive(Debug, Clone, Deserialize)]` + `#[serde(deny_unknown_fields)]`
- `pub struct ServerConfig { pub bind: String, pub threads: usize, pub graceful_shutdown: Duration, pub keepalive: Duration, pub max_connection: usize, pub admin: AdminConfig }`
- `pub struct AdminConfig { pub enabled: bool, pub bind: String }`
- `pub struct LoggingConfig { pub level: String, pub format: LogFormat, pub access_log: bool }`，`pub enum LogFormat { Text, Json }`
- `pub struct UpstreamConfig { pub connect_timeout: Duration, pub read_timeout: Duration, pub write_timeout: Duration, pub idle_timeout: Duration, pub health_check: HealthCheckConfig, pub tls: TlsConfig }`
- `pub struct TlsConfig { pub verify_cert: bool, pub verify_hostname: bool, pub ca_file: Option<PathBuf>, pub insecure_skip_verify: bool }`
- `pub struct ErrorConfig { pub format: ErrorFormat, pub request_id_header: String }`
- `pub struct RouteConfig { pub id: String, pub uri: String, pub order: i32, pub predicates: Vec<String>, pub filters: Vec<String>, pub upstreams: Vec<UpstreamConfigEntry>, pub metadata: RouteMetadata }`
- `pub struct UpstreamConfigEntry { pub addr: String, pub weight: usize }`
- `pub struct BreakerConfig { pub enabled: bool, pub failure_threshold: u32, pub wait_duration_in_open_state: Duration, pub half_open_permits: usize }`
- `GatewayConfig` 增加 `pub trusted_proxies: Vec<String>`、`pub breakers: HashMap<String, BreakerConfig>`
- `pub fn parse_duration(raw: &str) -> Result<Duration, String>`
- `pub struct ConfigErrors(Vec<ConfigError>)`，`pub struct ConfigError { pub path: String, pub message: String }`，`impl Display for ConfigErrors`（每条一行 `path: message`）
- `impl GatewayConfig { pub fn from_str(text: &str) -> Result<Self, ConfigErrors>; pub fn from_path(path: &Path) -> Result<Self, ConfigErrors> }`
- `fn validate(&self) -> Vec<ConfigError>`（在 `validate.rs`，规则见 spec D5）

**Steps:**

- [ ] **Step 1: 写失败的测试** `next-web-gateway/src/config/mod.rs` 末尾 `#[cfg(test)]`：

```rust
#[test]
fn parses_duration_units() {
    assert_eq!(parse_duration("500ms").unwrap().as_millis(), 500);
    assert_eq!(parse_duration("30s").unwrap().as_secs(), 30);
    assert_eq!(parse_duration("2m").unwrap().as_secs(), 120);
    assert_eq!(parse_duration("1h").unwrap().as_secs(), 3600);
    assert!(parse_duration("3 parsecs").is_err());
}

#[test]
fn rejects_unknown_fields_with_path() {
    let err = GatewayConfig::from_str("next:\n  gateway:\n    bogus: 1\n").unwrap_err();
    assert!(err.to_string().contains("bogus"), "{err}");
}

#[test]
fn accepts_both_root_keys_and_rejects_both_together() {
    assert!(GatewayConfig::from_str("next:\n  gateway:\n    routes: []\n").is_ok());
    assert!(GatewayConfig::from_str("gateway:\n  routes: []\n").is_ok());
    assert!(GatewayConfig::from_str("next:\n  gateway:\n    routes: []\ngateway:\n  routes: []\n").is_err());
}

#[test]
fn reports_all_validation_errors_at_once() {
    let yaml = "next:\n  gateway:\n    routes:\n      - id: a\n        uri: nope\n      - id: a\n        uri: lb://x\n";
    let err = GatewayConfig::from_str(yaml).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("routes[0].uri"), "{text}");
    assert!(text.contains("routes[1].id"), "{text}");
}
```

- [ ] **Step 2: 运行确认失败** —— `cargo test -p next-web-gateway config::` → 编译错误（模块不存在）。
- [ ] **Step 3: 实现** —— 按上面的 `Interfaces` 落结构体与默认值（默认值取 spec D5 表格）；`from_str` 流程：`serde_yaml::from_str::<Value>` → 取 `next.gateway` 或 `gateway`（两者都有则报错）→ `serde_path_to_error::deserialize` 收集反序列化错误 → `validate()` 追加 → 空则 Ok。`parse_duration` 用后缀匹配（`ms`/`s`/`m`/`h`），无后缀按毫秒并 `tracing::warn!`。
- [ ] **Step 4: 运行确认通过** —— 同上命令，4 passed。
- [ ] **Step 5: 补全 `validate()` 的测试** —— 每条 spec D5「以下情况启动失败」一个 test（共 12 条），断言 `err.to_string()` 含对应路径片段。
- [ ] **Step 6: 全绿 + clippy + 提交**

```bash
cargo test -p next-web-gateway && cargo clippy -p next-web-gateway --all-targets -- -D warnings
git add next-web-gateway/src/config next-web-gateway/src/lib.rs next-web-gateway/Cargo.toml
git commit -m "feat(config): replace properties with a validated gateway config schema"
```

---

### Task 2: `route` DSL —— 谓词/过滤器解析与 fail-fast

**Files:**
- Create: `next-web-gateway/src/filter/spec.rs`、`next-web-gateway/src/handler/predicate/spec.rs`
- Modify: `next-web-gateway/src/filter/gateway_filter.rs`（`From<&str>` → `TryFrom<&str, Error = ConfigError>`）、`next-web-gateway/src/handler/predicate/route_predicate_factory.rs`（同上）、`next-web-gateway/src/service/route_service.rs`

**Interfaces:**
- `pub struct FilterSpec { pub name: String, pub args: Vec<String> }`，`FilterSpec::parse(raw: &str) -> Result<FilterSpec, ConfigError>`（`Name=a, b` 逗号分隔；`Name=a:b` 兼容但 `warn!`）
- `pub trait GatewayFilter` 增加 `fn phase(&self) -> FilterPhase`（spec D17），`pub enum FilterPhase { RequestHeader, RequestBody, ResponseHeader, ResponseBody }`；`DefaultGatewayFilter` 与 `DefaultGatewayFilterChain` 在每一趟只执行匹配阶段的过滤器
- `impl TryFrom<&str> for DefaultGatewayFilter { type Error = ConfigError; }`
- `impl TryFrom<&str> for RoutePredicateFactory { type Error = ConfigError; }`
- `fn unknown_name_error(kind: &str, name: &str, known: &[&str]) -> ConfigError`（`message` 形如 `unknown filter "AddRequestHeaderX", did you mean "AddRequestHeader"? known filters: ...`）
- `fn expect_args<'a>(spec: &'a FilterSpec, min: usize, max: usize) -> Result<&'a [String], ConfigError>`

**Steps:**

- [ ] **Step 1: 写失败的测试**（`filter/spec.rs`）：

```rust
#[test]
fn unknown_filter_name_is_an_error_listing_known_names() {
    let err = DefaultGatewayFilter::try_from("AddRequestHeaderX=Foo: bar").unwrap_err();
    assert!(err.message.contains("AddRequestHeader"), "{}", err.message);
}

#[test]
fn comma_syntax_is_primary_and_colon_is_legacy() {
    let f = DefaultGatewayFilter::try_from("AddRequestHeader=X-A, 1, X-B, 2").unwrap();
    assert_eq!(f.header_pairs(), &[("X-A".into(), "1".into()), ("X-B".into(), "2".into())]);
    assert!(DefaultGatewayFilter::try_from("AddRequestHeader=X-A: 1").is_ok());
}

#[test]
fn invalid_numeric_argument_is_an_error_not_a_silent_default() {
    let err = DefaultGatewayFilter::try_from("SetStatus=abc").unwrap_err();
    assert!(err.message.contains("100"), "{}", err.message);
}
```

（谓词侧同构：`unknown_predicate_name_is_an_error`、`path_predicate_requires_at_least_one_pattern`。）

- [ ] **Step 2: 运行确认失败** —— `cargo test -p next-web-gateway spec::` → 编译错误。
- [ ] **Step 3: 实现** —— 解析器把 `Name=` 右侧按 `,` 切分并 `trim`（旧 `:` 语法在**第一个参数内含 `:` 且只有一个参数**时按 `k:v` 解释并 `warn!`）；给 `DefaultGatewayFilter` 加 `pub fn header_pairs(&self) -> &[(String, String)]`（仅 `AddRequestHeader`/`SetRequestHeader`/`AddResponseHeader`/`SetResponseHeader` 有值，其余返回 `&[]`）供测试与后续任务使用；`RoutePredicateService::from` 改为返回 `Result`，删除 `filter_map`/`filter` 丢弃逻辑（`route_service.rs:57-69`），并删除 `fallback_id` 的字符串前缀 hack（`:86-92`）。
- [ ] **Step 4: 运行确认通过**，`cargo test -p next-web-gateway` 全绿（旧测试中依赖 `From` 的用例同步改成 `TryFrom`）。
- [ ] **Step 5: 提交**

```bash
git commit -m "feat(route): fail fast on unknown filter and predicate names"
```

---

### Task 3: 谓词语义修正

**Files:** Modify `next-web-gateway/src/handler/predicate/{path,host,header,cookie,query,mehod,remote_addr,x_forwarded_remote_addr,weight,zoned_datetime}_route_predicate_factory.rs`、`next-web-gateway/src/util/str.rs`

**Interfaces:**
- `pub struct PathPattern { pub regex: Regex, pub variables: Vec<String> }`，`build_path_pattern` 支持 `{var}`、`{*var}`，返回捕获组名
- `impl RoutePredicate for PathRoutePredicateFactory`：匹配 `session.req_header().uri.path()`，命中后把变量写进 `session.req_header_mut().extensions`（键 `crate::handler::predicate::PATH_VARIABLES: &str`）
- `StrUtil::host_match(host: &str, pattern: &str) -> bool`：`*` = 单层标签，`**` = 零层或多层

**Steps:**
- [ ] **Step 1: 写失败的测试**（每个文件末尾，逐条对应 spec D11 谓词表）：

```rust
// path_route_predicate_factory.rs
#[test] fn path_matches_without_query_string() { /* /test/** 命中 /test?a=1 的 uri().path() == "/test" */ }
#[test] fn path_captures_named_variables() { /* /api/{version}/users 捕获 version=1 */ }
// host_route_predicate_factory.rs
#[test] fn host_star_matches_exactly_one_label() { /* *.example.com 命中 a.example.com，不命中 a.b.example.com */ }
#[test] fn host_double_star_matches_zero_or_more_labels() { /* **.example.com 命中 example.com 与 a.b.example.com */ }
// header_route_predicate_factory.rs
#[test] fn header_matches_any_of_several_values() { }
#[test] fn header_name_only_matches_when_present_even_with_trailing_comma() { }
#[test] fn header_regex_is_fully_anchored() { /* ^abc 不命中 xabc */ }
// cookie/query: first_value_is_used / regex_is_fully_anchored
// remote_addr: ipv4_mapped_ipv6_peer_matches_ipv4_cidr
// x_forwarded_remote_addr: untrusted_peer_ignores_xff / trusted_peer_uses_rightmost_entry
// weight: bucket_is_randomised_per_request
// zoned_datetime: value_is_trimmed / between_accepts_slash_separator / region_suffix_is_accepted
```

- [ ] **Step 2: 运行确认失败**、**Step 3: 实现**（按 spec D11 谓词表逐行）、**Step 4: 运行确认通过**
- [ ] **Step 5: 提交** —— `git commit -m "fix(predicate): align path, host, header and forwarded-addr semantics with spring cloud gateway"`

---

### Task 4: 请求侧过滤器语义修正

**Files:** Modify `next-web-gateway/src/filter/factory/{add_request_header,add_request_headers_if_not_present,add_request_parameter,map_request_header,prefix_path,preserve_host_header,remove_request_header,remove_request_parameter,request_header_size,request_size,rewrite_path,set_path,set_request_header,set_request_host_header,strip_prefix}.rs`、`next-web-gateway/src/util/path.rs`、`next-web-gateway/src/context/request_context.rs`

**Interfaces:**
- `RequestContext` 新增 `pub path_variables: HashMap<String, String>`、`pub original_host: Option<String>`、`pub original_scheme: &'static str`、`pub request_body_bytes: usize`、`pub request_id: String`
- `pub fn substitute_variables(template: &str, vars: &HashMap<String, String>) -> String`（`util/path.rs`）
- `PreserveHostHeaderFilter`：把 `ctx.original_host` 置为下游 `Host`，自身不再改头（Host 最终由 `upstream_request_filter` 统一写，见 Task 9）

**Steps:**
- [ ] **Step 1: 写失败的测试**（逐条对应 spec D11 过滤器表）：

```rust
#[test] fn add_request_parameter_preserves_duplicate_keys_and_order() { /* ?t=a&t=b 追加后两个都在 */ }
#[test] fn rewrite_path_replaces_all_matches() { /* REWRITE_PATH replace_all */ }
#[test] fn set_path_substitutes_path_variables() { /* SetPath=/v2/{version}/x */ }
#[test] fn request_header_size_counts_the_request_line() { }
#[test] fn request_size_limit_rejects_chunked_body_without_content_length() { }
```

- [ ] **Step 2/3/4** 同上；**Step 5 提交** `fix(filter): correct request-side filter semantics and legacy argument syntax`

---

### Task 5: 响应侧过滤器语义修正

**Files:** Modify `next-web-gateway/src/filter/factory/{add_response_header,dedupe_response_header,remove_response_header,rewrite_location_response_header,rewrite_response_header,redirect_to,secure_headers,set_response_header,set_status}.rs`、`next-web-gateway/src/filter/gateway_filter.rs`（`SaveSession`/`TokenRelay` 改为解析期报"不支持"）

**Interfaces:**
- `RewriteResponseHeaderFilter { header: String, regex: Regex, replacement: String }`（顺序按 SCG：`header, regexp, replacement`），在现value 上 `replace_all`
- `SecureHeadersFilter` 追加 SCG 默认集并用 `insert_header` 覆盖（不是"缺失才插"）
- `SetStatusFilter` 只在 `100..=599` 构造成功

**Steps:**
- [ ] **Step 1: 写失败的测试**

```rust
#[test] fn rewrite_response_header_uses_scg_argument_order_and_backreferences() { /* "X-Info, (\\w+), $1-kept" */ }
#[test] fn secure_headers_overrides_weakened_upstream_values() { }
#[test] fn set_status_rejects_out_of_range_values() { }
#[test] fn redirect_to_requires_parseable_status() { }
```

- [ ] **Step 2/3/4**；**Step 5 提交** `fix(filter): correct response-side filter semantics, argument order and security headers`

---

### Task 6: 限流与本地缓存

**Files:** Modify `next-web-gateway/src/filter/factory/request_rate_limiter.rs`、`next-web-gateway/src/filter/factory/local_response_cache.rs`、`next-web-gateway/src/service/route_service.rs`（删除路由级 `rate_limiter`）

**Interfaces:**
- `RequestRateLimiterFilter { limit: u32, window: Duration, limiter: Arc<Rate> }`；`fn client_key(ctx: &RequestContext) -> String`
- `LocalResponseCacheFilter { ttl: Duration, max_size_bytes: usize, manager: Arc<LocalResponseCacheManager> }`；`fn cache_key(req: &RequestHeader, route_id: &str) -> String`（含 `Vary` 头、`Cookie`、`Authorization`、`Accept-Encoding`、`Host` 的 hash）
- `LocalResponseCacheFilter::filter()` 自己完成命中/存储，`proxy` 里不再旁路调用

**Steps:**
- [ ] **Step 1: 写失败的测试**

```rust
#[test] fn rate_limiter_counts_each_request_once() { /* 同一 key 连续 2 次调用只 observe 2 次 */ }
#[test] fn rate_limiter_buckets_per_client_address() { /* 两个 IP 各自独立 */ }
#[test] fn rate_limiter_admits_exactly_limit_requests_per_window() { /* 第 limit+1 个 429 */ }
#[test] fn cache_key_separates_authorization_and_cookie() { }
#[test] fn cache_does_not_serve_no_cache_requests() { }
#[test] fn oversize_entry_evicts_only_itself() { }
```

- [ ] **Step 2/3/4**；**Step 5 提交** `fix(filter): make rate limiting per-client and the response cache safe`

---

### Task 7: 熔断器修正 + `CircuitBreaker` 过滤器

**Files:** Modify `next-web-gateway/src/circuit_breaker/{circuit_breaker_controller,circuit_breaker_service,circuit_breaker_service_manager}.rs`、`next-web-gateway/src/filter/factory/circuit_breaker.rs`、`next-web-gateway/src/filter/gateway_filter.rs`

**Interfaces:**
- `pub enum CircuitRejection { Open { retry_after: Duration } }`
- `CircuitBreakerController::try_acquire(&self) -> Result<(), CircuitRejection>`（含半开许可 `half_open_permits`，默认 1）
- `CircuitBreakerController::process(&self, outcome: Outcome)`，`pub enum Outcome { Success, Failure }`
- `CircuitBreakerServiceManager::from_config(&HashMap<String, BreakerConfig>) -> Self`
- `DefaultGatewayFilter::CircuitBreaker(CircuitBreakerFilter)`，`CircuitBreakerFilter { breaker: Arc<CircuitBreakerService> }`，`phase() = RequestHeader`
- 修掉 `circuit_breaker_controller.rs:42-312` 的 8 个 doctest：把 `use circuit_breaker::...` 改成 `use next_web_gateway::circuit_breaker::...`，并删掉已删除签名（`execute`）的示例

**Steps:**
- [ ] **Step 1: 写失败的测试**

```rust
#[tokio::test] async fn five_xx_response_counts_as_failure() { }
#[tokio::test] async fn half_open_admits_a_single_probe() { }
#[tokio::test] async fn disabled_breaker_never_opens() { }
#[test] fn circuit_breaker_is_a_parseable_filter() { /* "CircuitBreaker=main" */ }
#[test] fn undefined_breaker_id_fails_at_startup() { /* 错误信息含已定义 id 列表 */ }
```

- [ ] **Step 2/3/4** —— `response_filter` 里按 `upstream_response.status.as_u16() >= 500` 决定 `Outcome`；`fail_to_proxy` 补 `Failure`；删除 `suppress_error_log -> true`。
- [ ] **Step 5 提交** `fix(circuit-breaker): count 5xx as failure, limit half-open probes and wire the filter`

---

### Task 8: `upstream` 模块 —— URI 解析、异步 DNS、静态 LB、健康检查

**Files:** Create `next-web-gateway/src/upstream/{mod,uri,dns_cache,resolver,load_balancer}.rs`；Delete `next-web-gateway/src/service/jingyue_service.rs`

**Interfaces:**
- `pub struct UpstreamUri { pub scheme: UpstreamScheme, pub host: String, pub port: u16, pub path_prefix: String }`，`UpstreamScheme::{Http,Https,Lb}`，`UpstreamUri::parse(&str) -> Result<Self, ConfigError>`
- `pub struct DnsCache { /* capacity 1024, ttl 30s */ }`，`async fn resolve(&self, host: &str, port: u16) -> Result<Vec<SocketAddr>, UpstreamError>`
- `pub trait UpstreamResolver: Send + Sync { fn select(&self, route_id: &str, key: &[u8]) -> Option<SocketAddr>; }`（Phase 3 的注册中心挂这里）
- `pub struct StaticLoadBalancer`，`StaticLoadBalancer::build(route: &RouteConfig) -> Result<Self, ConfigError>`，内部 `LoadBalancer<RoundRobin>` + `Arc<HealthRegistry>`
- `pub struct UpstreamSelection { pub addr: SocketAddr, pub sni: String, pub tls: bool, pub group_key: u64 }`
- `pub fn build_peer(sel: &UpstreamSelection, cfg: &UpstreamConfig, route: &RouteConfig) -> HttpPeer`（`HttpPeer::new(addr, tls, sni)` + `options.*`，绝不传空 sni）

**Steps:**
- [ ] **Step 1: 写失败的测试**

```rust
#[test] fn parses_http_https_and_lb_uris() { }
#[test] fn rejects_uri_without_scheme_or_with_unknown_scheme() { }
#[test] fn lb_uri_requires_a_non_empty_upstream_list() { }
#[test] fn https_uri_yields_non_empty_sni() { /* build_peer(...).sni == "example.com" */ }
#[test] fn verify_cert_defaults_to_true_and_insecure_flag_disables_both() { }
#[tokio::test] async fn dns_cache_resolves_localhost_once() { }
#[tokio::test] async fn round_robin_rotates_across_two_upstreams() { }
```

- [ ] **Step 2/3/4** —— 健康检查用 `pingora::lb::health_check::TcpHealthCheck` + `HealthCheckService` 注册为 background service（Task 11 挂到 `Server`）。
- [ ] **Step 5 提交** `feat(upstream): add async dns cache and static load balancing with health checks`

---

### Task 9: `proxy` 主流程 + 错误映射

**Files:** Create `next-web-gateway/src/proxy/{mod,context,error_response,filters}.rs`；Delete `next-web-gateway/src/application/next_gateway_application.rs`、`src/context/request_context.rs`（内容迁进 `proxy/context.rs`）、`src/server/*`

**Interfaces:**
- `pub struct GatewayProxy { /* routes: RouteService, upstreams: UpstreamService, config: Arc<GatewayConfig>, metrics: Arc<Metrics> */ }`
- `impl ProxyHttp for GatewayProxy`，`type CTX = ProxyContext`
- `pub struct ProxyContext { pub request_id: String, pub route_id: Option<String>, pub path_variables: HashMap<String,String>, pub original_host: Option<String>, pub selected: Option<UpstreamSelection>, pub direct_response: Option<DirectResponse>, pub buffer: Option<BodyBuffer>, pub started: Instant }`
- `pub fn status_for_error(e: &pingora::Error) -> u16`（spec D6 表）
- `pub fn render_error(cfg: &ErrorConfig, status: u16, route: Option<&str>, request_id: &str) -> (ResponseHeader, Bytes)`
- `pub async fn apply_response_filters(...)`，也被 `fail_to_proxy` 调用，保证网关自产响应过响应过滤器链

**Steps:**
- [ ] **Step 1: 写失败的测试**

```rust
#[test] fn maps_no_route_to_404_and_open_circuit_to_503() { }
#[test] fn json_error_body_carries_request_id_and_route() { }
#[test] fn text_error_body_is_a_single_line() { }
#[test] fn buffered_body_is_capped_and_falls_back_to_streaming() { }
```

- [ ] **Step 2: 实现要点**（spec D6/D9/D10/D17）—— `request_filter`：生成/沿用 `X-Request-Id` → 路由匹配（无 → 404 直接响应）→ 限流 → 熔断 `try_acquire` → 失败即短路；`upstream_peer`：`UpstreamResolver::select` → `build_peer`，无可用上游 → 503 + `Retry-After`；`upstream_request_filter`：剥离 hop-by-hop、追加 `X-Forwarded-For/-Proto/-Host`、写 `Host`（除非 `ctx.original_host` 被 `PreserveHostHeader` 保留）、跑 `FilterPhase::RequestHeader` 链；`request_body_filter`：跑 `FilterPhase::RequestBody` 链（`RequestSize` 累计计数，超限即 413 中断）；`response_filter`：按 `status >= 500` 记熔断结果、跑 `FilterPhase::ResponseHeader` 链、决定是否缓冲（上限 `max_buffer_bytes`，超限则放弃改写走流式并 `warn!`）；`response_body_filter`：**不再 `block_on`**，body 过滤器改为同步 trait；`logging`：Task 10 填。网关自产响应按 D17：请求侧链停止，`fail_to_proxy` 里跑 `ResponseHeader` + `ResponseBody` 两趟后再写回。
- [ ] **Step 3: 运行确认通过**；**Step 4: 提交** `feat(proxy): rewrite the proxy pipeline with correct error mapping and bounded buffering`

---

### Task 10: `metrics` + `admin` 监听器

**Files:** Create `next-web-gateway/src/metrics/{mod,registry,exposition}.rs`、`next-web-gateway/src/admin/{mod,service}.rs`

**Interfaces:**
- `pub struct Metrics { /* AtomicU64 字段 */ }`，`Metrics::observe_request(route: &str, method: &str, status: u16, elapsed: Duration)`、`Metrics::render_prometheus(&self, out: &mut String)`
- `pub struct AdminService { config: Arc<GatewayConfig>, routes: Arc<RouteService>, metrics: Arc<Metrics>, started: Instant }`，`impl ProxyHttp for AdminService`
- `AdminService::routes_snapshot(&self) -> Vec<RouteSnapshot>`，`pub struct RouteSnapshot { pub id: String, pub uri: String, pub upstreams: Vec<String>, pub predicates: Vec<String>, pub filters: Vec<String> }`

**Steps:**
- [ ] **Step 1: 写失败的测试**

```rust
#[test] fn prometheus_exposition_has_help_type_and_labels() { /* 含 "# TYPE gateway_requests_total counter" */ }
#[test] fn request_counter_is_incremented_once_per_request() { }
#[test] fn admin_rejects_unknown_paths_with_404() { }
```

- [ ] **Step 2/3/4** —— `/health/live`、`/health/ready`、`/metrics`、`/routes`、`/version`；`logging` 钩子在 `GatewayProxy` 里补一条结构化 access log（spec D10 字段顺序）并调用 `metrics.observe_request`。**一次请求只调一次**（Review Focus 5）。
- [ ] **Step 5 提交** `feat(observability): add access logs, prometheus metrics and an admin listener`

---

### Task 11: `Gateway` 门面 + 二进制 + CLI

**Files:** Create `next-web-gateway/src/gateway.rs`、`next-web-gateway/src/bin/next_web_gateway.rs`；Delete `next-web-gateway/src/application/gateway_application.rs`；Modify `next-web-gateway/Cargo.toml`（`[[bin]]`、`clap`、`uuid`）

**Interfaces:**
- `pub struct Gateway { config: GatewayConfig, config_path: Option<PathBuf> }`
- `Gateway::from_path(&Path) -> Result<Self, ConfigErrors>`、`Gateway::from_str(&str) -> Result<Self, ConfigErrors>`、`Gateway::check(&self) -> Vec<RouteSnapshot>`、`Gateway::run(self) -> !`
- `pub struct Cli { config: PathBuf, bind: Option<String>, admin_bind: Option<String>, log_level: Option<String>, log_format: Option<LogFormat>, check: bool }`（`clap::Parser`，参数表见 spec D4）
- 退出码契约：`0` 正常 / `1` 运行时错误 / `2` 配置错误（`--check` 非法配置也是 `2`）

**Steps:**
- [ ] **Step 1: 写失败的测试**（`gateway.rs`：`check` 返回路由快照；`bin` 用 `Cli::try_parse_from` 测参数解析与 `--check` 退出码）
- [ ] **Step 2: 实现** —— `run()`：初始化 tracing（`EnvFilter`，`json` 时 `.json()`）→ 构建 `GatewayProxy` + `AdminService` + 健康检查 background service → `Server::new(Opt::default())`（错误不 `unwrap`）→ `bootstrap()` → 两个 `http_proxy_service` 分别 `add_tcp` 到 `server.bind` / `server.admin.bind` → `run_forever()`。配置错误必须 `eprintln!` 后 `std::process::exit(2)`，运行时错误 `exit(1)`。
- [ ] **Step 3/4** —— `cargo run -p next-web-gateway -- --config next-web-gateway/application.yaml --check` 退出码 0。
- [ ] **Step 5 提交** `feat(bin): add the next-web-gateway binary with cli, config check and graceful shutdown`

---

### Task 12: 集成测试 harness

**Files:** Create `next-web-gateway/tests/support/{mod,upstream,client,gateway,tls}.rs`、`next-web-gateway/tests/harness_smoke.rs`；Modify `next-web-gateway/Cargo.toml`（`[dev-dependencies]` 加 `tokio-rustls`、`rustls`、`rustls-pemfile`，全部取自 workspace）

**Interfaces:**
- `pub struct MockUpstream { pub addr: SocketAddr, pub requests: Arc<Mutex<Vec<RecordedRequest>>> }`，`MockUpstream::start(behavior: Behavior) -> Self`，`pub enum Behavior { Respond(u16, Vec<(String,String)>, Vec<u8>), Echo, Chunked(Vec<Vec<u8>>), Close, Hang }`
- `MockUpstream::start_tls(behavior: Behavior, tls: &TlsFixture) -> Self`（`tests/support/tls.rs`：用 `tokio_rustls::TlsAcceptor` + `tests/fixtures/tls/server.pem/.key` 提供 HTTPS 上游）
- `pub struct TestGateway { pub base: String, pub admin: Option<String>, shutdown: watch::Sender<bool> }`，`TestGateway::start(yaml: &str) -> Self`，`impl Drop` 里 `shutdown.send(true)`
- `pub fn get(base: &str, path: &str, headers: &[(&str,&str)]) -> HttpResponse`，`pub struct HttpResponse { pub status: u16, pub headers: Vec<(String,String)>, pub body: Vec<u8> }`

**Steps:**
- [ ] **Step 1: 写失败的测试** `harness_smoke.rs`：起一个 `Behavior::Respond(200, [], b"ok".to_vec())` 上游 + 一条 `Path=/ping` 路由，断言 `get(&gw.base, "/ping", &[]).body == b"ok"`。
- [ ] **Step 2: 生成 TLS 夹具** —— 本机 `PATH` 里没有 `openssl`，但 Git 自带了一个：`D:\DevApp\Git\usr\bin\openssl.exe`。**缺陷：** 它是 MSYS2 构建，运行时要创建信号管道，在 DSH 沙箱里会报 `couldn't create signal pipe, Win32 error 5`，所以这条命令必须在**沙箱外**跑一次（和 zlib-ng 首次构建同理）。生成 `tests/fixtures/tls/{ca.pem,server.pem,server.key,selfsigned.pem,selfsigned.key}`（SAN = `DNS:localhost,IP:127.0.0.1`），把生成脚本 `tests/fixtures/tls/generate.sh` 与 `README.md`（说明来源、有效期 10 年、重生成方式）一起提交；夹具本身**提交进仓库**，不引 `rcgen`。不要用 `New-SelfSignedCertificate`（写证书存储会被拒：`NTE_PERM`），也不要用 .NET `ExportCertificatePem`（本机是 Windows PowerShell 5.1，没有该 API）。
- [ ] **Step 3: 实现** —— 端口探测（`TcpListener::bind("127.0.0.1:0")` 取端口后释放）；`Service::start_service(None, shutdown, 1, ServiceReadyNotifier::new(tx))`（unix 传 `None`，Windows 无该参数）；客户端手写 HTTP/1.1（支持 `Content-Length` 与 `Transfer-Encoding: chunked` 解析）。
- [ ] **Step 4/5** —— `harness_smoke` 通过（此时 Task 1–11 已就绪）。
- [ ] **Step 6 提交** `test(gateway): add a wire-level integration harness with mock http and https upstreams`

---

### Task 13: E2E 用例

**Files:** Create `next-web-gateway/tests/{routing,rewrite,limits,lb,observability,errors,tls_upstream,cli}.rs`

**Steps:**
- [ ] **Step 1: 写全部用例（先全部失败或在 Task 12 之后确认最后一个失败）**

```rust
// routing.rs
#[test] fn unmatched_path_returns_404() { }
#[test] fn order_decides_between_overlapping_routes() { }
// rewrite.rs
#[test] fn strip_prefix_and_add_request_header_reach_the_upstream() { }
#[test] fn rewrite_path_keeps_query_string() { }
#[test] fn preserve_host_header_keeps_the_original_host() { }
// limits.rs
#[test] fn rate_limiter_returns_429_with_retry_after() { }
#[test] fn request_size_returns_413_for_chunked_over_limit() { }
// lb.rs
#[test] fn round_robin_spreads_requests_across_two_upstreams() { }
#[test] fn dead_upstream_returns_503() { }
#[test] fn unreachable_upstream_returns_502() { }
// observability.rs
#[test] fn access_log_has_one_line_per_request() { }
#[test] fn metrics_endpoint_counts_the_request_once() { }
#[test] fn health_ready_flips_when_all_upstreams_die() { }
#[test] fn request_id_is_generated_and_echoed() { }
// errors.rs
#[test] fn upstream_500_passes_through_and_trips_the_breaker() { }
#[test] fn gateway_error_body_is_json_and_carries_request_id() { }
// tls_upstream.rs
#[test] fn https_upstream_with_configured_ca_succeeds() { }
#[test] fn https_upstream_without_ca_fails_with_502() { }
#[test] fn https_upstream_with_insecure_skip_verify_succeeds() { }
// cli.rs —— 用 env!("CARGO_BIN_EXE_next-web-gateway") 起子进程
#[test] fn check_accepts_the_shipped_application_yaml() { }
#[test] fn check_exits_2_on_invalid_config_and_prints_the_path() { }
#[test] fn missing_config_file_exits_2() { }
#[test] fn help_exits_0() { }
```

- [ ] **Step 2/3/4**；**Step 5 提交** `test(gateway): cover routing, rewrites, limits, load balancing, tls, cli and observability end to end`

---

### Task 14: 文档、Docker、样例配置、零 warning 收敛

**Files:** Create `next-web-gateway/Dockerfile`、`next-web-gateway/examples/config/{minimal,lb,https-upstream}.yaml`；Rewrite `next-web-gateway/README.md`、`next-web-gateway/application.yaml`；Modify 根 `README.md`、`README.zh-CN.md`

**Steps:**
- [ ] **Step 1: 重写 `application.yaml`** 为能直接 `--check` 通过的样例（spec D12）
- [ ] **Step 2: 验证样例** —— `cargo run -p next-web-gateway -- --config next-web-gateway/application.yaml --check` 退出码 0；三个 `examples/config/*.yaml` 各自同上。
- [ ] **Step 3: 写 README** —— 概览、快速开始（docker + 本地）、配置参考（逐字段表，含 spec D5 全表）、谓词表、过滤器表、运维（健康检查/指标/日志/退出码）、与 SCG 的差异清单、Phase 2/3 路线。**过滤器表和代码里的名称必须一一对应**（用一个 `#[test] fn readme_lists_every_filter()` 断言 README 中出现每个 `DefaultGatewayFilter` 变体名）。
- [ ] **Step 4: 写 Dockerfile** —— 多阶段，`EXPOSE 8080 9001`，`ENTRYPOINT ["/usr/local/bin/next-web-gateway"]`。
- [ ] **Step 5: 收敛 warning** —— `cargo clippy -p next-web-gateway --all-targets -- -D warnings` 为 0；`cargo check` 为 0 warning；`cargo test -p next-web-gateway --doc` 全绿。
- [ ] **Step 6: 提交** `docs(gateway): document configuration, filters and operations; add docker image`
