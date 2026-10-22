# next-web-gateway 生产可用设计（Spring Cloud Gateway 对应物）

**状态：** 待评审
**日期：** 2026-10-22
**范围：** Phase 1（可上线）。Phase 2 / Phase 3 在本文件末尾作为路线图，不在本次实施计划内。

---

## 1. 意图与缘由

`next-web-gateway` 是 workspace 里唯一以 `pingora` 实现的流量入口 crate，目标是成为 Spring Cloud Gateway（下称 SCG）在 Rust 生态里的对应物：用 YAML + 字符串 DSL 声明 `routes / predicates / filters`，不改代码即可完成路由、改写、限流、熔断、缓存。

当前状态（两个只读审计交叉验证，结论一致）：**骨架完整、方向正确、但不可上线**。

- 不可启动：没有 `[[bin]]`、没有示例、workspace 无消费者；`GatewayApplication::run()` 在配置加载失败时**静默 return**（退出码 0、不监听端口）；`lib.rs` 把所有模块设为私有，外部连 `NextGatewayApplication::new` 的参数类型都无法命名。
- 自带 `application.yaml` 加载不了：文件用 `next.gateway`，加载器要求顶层 `gateway`；且 `filters` 是必填字段而两个样例路由都没写。
- `lb://` 是桩：`JingYueService::select` 忽略服务名，恒返回 `127.0.0.1:3000`。
- `https://` 上游实际不校验证书：`HttpPeer::new(host, true, "")` 的空 SNI 让 pingora 走 `VerificationMode::SkipAll`。
- 普通配置会 panic：`HttpPeer::new` 内部阻塞 `to_socket_addrs().unwrap()`，`https://host`（无端口）、`http://host/path`、NXDOMAIN 全都崩在请求路径上。
- 错误语义全塌成 400：无路由、熔断打开、无上游都是 400。
- 熔断器把 5xx 当成功；`CircuitBreaker` 过滤器根本没进 enum，靠字符串前缀 hack 提取 id。
- 限流按路由而非客户端，且被请求/响应两趟重复计数。
- 配置错误 fail-open：非法 predicate 被丢弃 → 约束消失、路由变宽。
- 无访问日志、无指标、`suppress_error_log` 恒 `true` 把所有代理错误静音。

一句话：**它不是 SCG 的对应物，也部署不了。** 本设计定义把它变成"能上线、语义正确、可观测、可回归"的网关。

---

## 2. 目标

**G1 可启动、可运维。** 有可执行文件、有 CLI、配置错误以非零退出码失败并打印带路径的错误列表；有健康检查端点、访问日志、Prometheus 指标；能优雅退出；能放进容器。

**G2 语义正确。** 已存在的 11 个谓词与 31 个过滤器按 SCG 语义工作；错误码分类正确（404 / 429 / 413 / 431 / 502 / 503 / 504）；配置不再 fail-open。

**G3 上游可信。** `https://` 上游默认校验证书与主机名；`lb://` 走真实的 pingora 负载均衡（静态 upstream 列表 + 可选 TCP 健康检查）；Host 默认改写为上游 authority。

**G4 有回归网。** 单元测试覆盖每个修复点；`tests/` 里有真实 wire 级集成测试（自建 upstream + 自建 HTTP/1.1 客户端 + 真实 pingora 实例）。

**G5 有文档。** `next-web-gateway/README.md` 成为使用文档（配置参考、谓词/过滤器表、运维手册），根 README 加一节。

## 3. 非目标（Phase 1 明确不做）

- 热重载 / 配置中心下发（Phase 3）
- WebSocket 代理、网关侧 TLS 终止（Phase 3）
- 真实注册中心客户端（Nacos / Consul / K8s API）；Phase 1 只做静态 upstream + 保留 `Discovery` 接口（Phase 3）
- `ModifyRequestBody` / `ModifyResponseBody` / `CacheRequestBody` / `Retry` / `FallbackHeaders` / `JsonToGrpc` 过滤器（Phase 2）
- 全局过滤器层（`X-Forwarded-*` 除外，见 D9）、CORS 完整实现、actuator 全量（Phase 2）
- 分布式追踪（OpenTelemetry）、指标上报（Phase 2）
- Resilience4j 的滑动窗口/慢调用语义（Phase 2 再做 failure-rate + 半开许可）

## 4. 反目标（做出来就是失败）

- **A1 静默降级。** 任何拼错的 filter / predicate 名、任何非法参数，都不能退化成 no-op 继续跑。必须启动失败。
- **A2 错误静音。** 不允许 `suppress_error_log` 恒真、不允许 5xx 不留日志。
- **A3 生产路径 panic。** `unwrap` / `expect` / `panic!` / `unreachable!` 出现在请求路径或配置加载路径（workspace lint `unsafe_code = "forbid"` 已有，panic 需人工守）。
- **A4 新增未使用依赖。** 每加一个 crate 必须当次用上。
- **A5 无证据的完成宣告。** 每个任务必须附可执行命令与实际输出。

## 5. 约束

- Rust edition 2024，`rust-version = "1.98.1"`（workspace 已定）。
- 构建统一 `CARGO_TARGET_DIR=D:\DevProject\Rust\next-web\.cargo-target`；本机 `pingora → flate2(zlib-ng)` 需要 CMake/MSBuild，首次构建须在沙箱外执行一次并把 `TEMP`/`TMP` 指向工作区内可写目录（`.cargo-target\tmp`），之后沙箱内可正常增量构建。
- 所有 `pub` 项必须有英文文档注释；`missing_docs = "warn"`、`unreachable_pub = "warn"` 为 workspace lint。
- 生产代码禁止 `println!`，日志一律 `tracing`。
- 提交信息 `type(scope): subject`，英文正文，一任务一提交。
- 每个任务收尾必须：`cargo test -p next-web-gateway` 全绿、`cargo clippy -p next-web-gateway --all-targets -- -D warnings` 0 警告、`cargo test -p next-web-gateway --doc` 全绿（当前 8 个 doctest 因引用不存在的 crate 而失败，必须一并修掉）。
- 可用的既有 workspace 依赖（直接引用，不许改版本）：`clap 4.5`（derive）、`uuid 1.16`（v4）、`serde_path_to_error 0.1`、`dashmap`、`arc-swap`、`parking_lot`、`rustls 0.23`、`tokio-rustls 0.26`、`rustls-pemfile 2.2`。

---

## 6. 已定的决策

标 **[决定]** 的是我替你做的主意；标 **[你的]** 是你在会话里定的。

### D1 [你的] 交付方式：分阶段
Phase 1 可上线 → Phase 2 SCG 特性对齐 → Phase 3 热重载/WebSocket/TLS 终止/发现。每阶段独立验收。

### D2 [你的] 负载均衡：静态 upstream + pingora LB
`lb://name` 的实例写在路由的 `upstreams:` 列表里，用 `pingora::lb::LoadBalancer<RoundRobin>`（带 `weight`）做选择，可选 TCP 健康检查。同时定义 `UpstreamResolver` 接口，Phase 3 的注册中心实现挂在这里。

### D3 [你的] 允许破坏性变更
配置前缀统一 `next.gateway`；`predicates`/`filters` 改为可选；公开 `filter` / `handler` / `context` / `properties` 模块作为扩展点；未知字段直接报错。

### D4 [决定] 入口形态：`Gateway` 库对象 + `next-web-gateway` 二进制
- `src/gateway.rs`：`pub struct Gateway`，`Gateway::from_path(&Path) -> Result<Gateway, GatewayConfigError>`、`Gateway::from_str(&str)`、`Gateway::run(self) -> !`、`Gateway::check(self) -> Report`。
- `src/bin/next_web_gateway.rs`：`[[bin]] name = "next-web-gateway"`，`clap` derive 解析 CLI，调用 `Gateway`。
- 移除 `GatewayApplication` trait 与 `GatewayApplicationProperties::default()` 里"自建 tokio runtime + panic"的实现。`impl Default for GatewayApplicationProperties` 直接删除。

CLI 契约（全部可被同名环境变量覆盖）：

| 参数 | 环境变量 | 默认 |
|---|---|---|
| `--config <PATH>` | `NEXT_WEB_GATEWAY_CONFIG` | `./application.yaml` |
| `--bind <ADDR>` | `NEXT_WEB_GATEWAY_BIND` | 配置值，否则 `0.0.0.0:8080` |
| `--admin-bind <ADDR>` | `NEXT_WEB_GATEWAY_ADMIN_BIND` | 配置值，否则 `127.0.0.1:9001` |
| `--log-level <LEVEL>` | `RUST_LOG` | 配置值，否则 `info` |
| `--log-format <text\|json>` | `NEXT_WEB_GATEWAY_LOG_FORMAT` | `text` |
| `--check` | — | 只校验配置，`0` 合法 / `2` 非法 |
| `-d/--daemon`、`-t`、`-u` | — | 透传给 pingora `Opt`（守护进程、配置自检、优雅升级） |

启动失败**必须**非零退出码：`1` = 运行时错误，`2` = 配置错误。

### D5 [决定] 配置 schema v1

```yaml
next:
  gateway:
    server:
      bind: "0.0.0.0:8080"
      threads: 0                 # 0 = pingora 自动
      graceful_shutdown: 30s
      keepalive: 60s
      max_connection: 0          # 0 = 不限制
      admin:
        enabled: true
        bind: "127.0.0.1:9001"
    logging:
      level: info
      format: text               # text | json
      access_log: true
    upstream:                    # 全局默认，可被 route.metadata.client 覆盖
      connect_timeout: 2s
      read_timeout: 30s
      write_timeout: 30s
      idle_timeout: 60s
      health_check:
        enabled: true
        interval: 5s
      tls:
        verify_cert: true
        verify_hostname: true
        ca_file: null            # PEM 路径
        insecure_skip_verify: false
    error:
      format: json               # json | text
      request_id_header: X-Request-Id
    trusted_proxies: []          # CIDR 列表；空 = 谁都不信（见 D15）
    breakers:                    # 熔断器定义，按 id 全局共享（见 D8）
      main:
        enabled: true
        failure_threshold: 10
        wait_duration_in_open_state: 5s
        half_open_permits: 1
    routes:
      - id: users
        uri: "lb://user-service"
        order: 100               # 省略 = i32::MAX
        predicates:
          - Path=/api/users/**
          - Method=GET,POST
        filters:
          - StripPrefix=1
          - AddRequestHeader=X-Gateway, next-web
        upstreams:               # 仅 lb:// 必填
          - addr: "10.0.0.11:8080"
            weight: 2
          - addr: "10.0.0.12:8080"
            weight: 1
        metadata:
          client:
            connect_timeout: 1s
            read_timeout: 10s
            write_timeout: 10s
```

规则：
- 根键同时接受 `next.gateway` 与旧的顶层 `gateway`，后者加载时 `warn!` 一条弃用提示。两者同时出现 → 配置错误。
- 所有配置结构体 `#[serde(deny_unknown_fields)]`。
- `predicates` / `filters` `#[serde(default)]`。
- 时长字段接受 `500ms` / `30s` / `2m` / `1h`（自写解析器，不引依赖），也接受裸整数（毫秒，向后兼容旧 `metadata.client.*`），但裸整数加载时 `warn!`。
- **校验阶段收集全部错误再一次性返回**，用 `serde_path_to_error` 给出 `routes[2].filters[1]` 级别的路径。
- 以下情况启动失败（不是 warn）：未知 predicate/filter 名；正则编译失败；CIDR 非法；`Weight` 权重 ≤ 0 或组内总权重为 0；`Between` 缺第二个时间；`uri` 无 scheme 或 scheme 不在 `{http,https,lb}`；`lb://` 无 `upstreams` 或 `upstreams` 为空；`http/https` 却给了 `upstreams`；route id 重复；`Method` 非法 token;`SetStatus` 非 100–599;`StripPrefix` 非整数；`RequestRateLimiter` 非正整数；`RequestSize`/`RequestHeaderSize` 非正整数。
- `lb://name` 的 `name` 只用于日志与指标，实例地址以 `upstreams` 为准。

### D6 [决定] 错误语义

| 情形 | 状态码 | 备注 |
|---|---|---|
| 无路由命中 | 404 | SCG 行为 |
| 熔断打开 | 503 | 带 `Retry-After`（取 `wait_duration_in_open_state`） |
| `lb://` 无健康上游 | 503 | |
| 上游连接失败 / 连接被重置 | 502 | |
| 上游或网关超时 | 504 | pingora 的 `ErrorType::ConnectTimedout` / `ReadTimedout` / `WriteTimedout` / `TLSHandshakeTimedout`（pingora-error 0.9 **没有** `UpstreamTimedOut`） |
| 请求被过滤器拒绝 | 过滤器给定值 | 429 / 413 / 431 等 |
| 配置 / 内部错误 | 500 | |

- 错误响应体统一由 `error.format` 决定：
  - `json`: `{"timestamp":"…","status":503,"error":"Service Unavailable","message":"...","route":"users","requestId":"..."}`
  - `text`: `503 Service Unavailable\n`
- 网关自己生成的响应（404/429/503/…）**必须也走一遍响应过滤器链**（`SecureHeaders`/`AddResponseHeader`/`RemoveResponseHeader` 要对它们生效）。实现方式：`fail_to_proxy` 里构造 `ResponseHeader` 后调用一次 response-header 过滤器链，再写回客户端。
- `X-Request-Id`：若下游带了 `error.request_id_header` 指定的头则沿用，否则生成 UUIDv4；透传给上游，并写回客户端响应头，同时出现在 access log 与错误响应体里。

### D7 [决定] 上游选择

- `uri` 解析为 `{scheme, authority, path_prefix}`。`path_prefix` 在 Phase 1 **不自动拼接**（与现状一致，避免破坏已有路由），只在 `metadata` 里记录，供 Phase 2 的 `RouteToRequestUrl` 使用。
- `http://` / `https://`：`tokio::net::lookup_host` 异步解析，结果进进程内 DNS 缓存（TTL 30s，容量 1024），再用 `HttpPeer::new(SocketAddr, tls, sni)` —— `SocketAddr` 实现 `ToSocketAddrs`，不会触发阻塞 DNS，`unwrap` 也不会被踩到。
- `lb://`：按 route id 持有 `LoadBalancer<RoundRobin>`，`select(key, 256)`；`key` 用「客户端 IP + 路径」的 hash（Phase 2 支持 consistent-hash 时复用）。
- `sni` 恒为 authority 的 host（去掉端口），**绝不传空串**。`options.verify_cert` / `verify_hostname` 来自 `upstream.tls`，`insecure_skip_verify: true` 时两者置 false 并 `warn!` 一条。
- 当前构建用的是 pingora 默认的 **openssl** 后端（`pingora-core` 的 `CaType = Box<[X509]>`，rustls 后端则是 `[WrappedX509]`）。因此 `ca_file` 用 `openssl::x509::X509::stack_from_pem(&pem)` → `Box<[X509]>` 赋给 `PeerOptions::ca`；在 README 注明该字段只在 openssl 后端可用。系统根证书由 `openssl-probe` 自动探测，无需配置。client cert（mTLS）留到 Phase 3。
- `alpn` 默认 `H1`。HTTP/2 上游 Phase 2 再开。
- `group_key` = route id，保证不同路由不复用连接。
- 超时优先级：`route.metadata.client.*` > `gateway.upstream.*` > pingora 默认。
- Host 改写：默认把上游请求的 `Host` 设为上游 authority；`PreserveHostHeader` 过滤器改为「把原始 Host 存进 ctx，由 app 在 `upstream_request_filter` 末尾统一决定」。这是对现有 `PreserveHostHeader`（实际是 no-op）的修正。

### D8 [决定] 熔断语义（Phase 1 只修正确性，不改算法）

- `failure_threshold` 语义按**连续失败次数**（保留现状），但把 `CircuitBreakerProperties` 的注释与字段名改对（当前注释说是百分比）。配置位置见 D16。
- 触发失败的判定改为：上游连接失败、超时、5xx 响应。`response_filter` 里按 `status >= 500` 决定 `process(false)`，否则 `process(true)`。
- `fail_to_proxy` 里补 `process(false)`（现在完全没有）。
- 半开状态加**并发许可**：`CircuitBreakerController` 增加 `AtomicUsize` 的 `half_open_permits`（默认 1，可配），`state()` 之外新增 `try_acquire()`，`upstream_peer` 里用它决定放行与否。
- `enabled: false` 真正生效（现在 `active` 字段被写入但从不读）。
- `CircuitBreaker` 成为**真正的过滤器**：加入 `DefaultGatewayFilter` enum 与 `TryFrom<&str>` 解析，参数为 breaker id，`phase() = RequestHeader`（见 D17）。过滤器自身只做「若打开则直接返回 503 兜底响应 + `Retry-After`」；`upstream_peer` 里的字符串前缀 hack（`route_service.rs:86-92`，含 `split_once('=').unwrap()` panic）删除。
- `CircuitBreakerFallback`（fallback 响应体）Phase 2 做；Phase 1 只保留 503 + `Retry-After`。

### D9 [决定] Phase 1 的最小全局行为

不做完整全局过滤器层，但以下必须做，它们是"能上线"的底线：

1. **`X-Forwarded-For`**：在已有值后追加 `client_ip`；没有则新建。
2. **`X-Forwarded-Proto`**：下游 scheme（`http`，Phase 3 网关 TLS 终止后为 `https`）。
3. **`X-Forwarded-Host`**：下游 `Host`。
4. **`X-Request-Id`**：见 D6。
5. **hop-by-hop 剥离**：`Connection`、`Keep-Alive`、`Proxy-Authenticate`、`Proxy-Authorization`、`TE`、`Trailer`、`Transfer-Encoding`、`Upgrade` 在转发前从请求头移除（`Transfer-Encoding` 由 pingora 自己管）。
6. **请求体大小上限**：`RequestSize` 从"只看 `Content-Length`"改为在 `request_body_filter` 里累计字节数并强制中断，解决 chunked 绕过。

### D10 [决定] 可观测

访问日志（`ProxyHttp::logging`）每条一行：
```
ts=2026-10-22T21:30:00.123+08:00 level=INFO request_id=… client=1.2.3.4:5678 method=GET path=/api/users/1 status=200 route=users upstream=10.0.0.11:8080 latency_ms=12 upstream_ms=9 bytes=1234 reused=false error=-
```
`--log-format json` 时输出 JSON 行（字段同名）。

指标（手写原子计数器 + Prometheus 文本格式，不引依赖）：
- `gateway_requests_total{route,method,status}` counter
- `gateway_request_duration_seconds{route}` histogram（桶：1,5,10,25,50,100,250,500,1000,2500,5000,10000 ms）
- `gateway_upstream_errors_total{route,kind}` counter，`kind ∈ {connect,timeout,reset}`
- `gateway_circuit_state{breaker}` gauge（0=closed,1=half-open,2=open）
- `gateway_local_cache_hits_total` / `gateway_local_cache_misses_total` counter

admin 监听器（独立 `ProxyHttp` 实现，单独 `http_proxy_service`）：
- `GET /health/live` → 200 `{"status":"live"}`
- `GET /health/ready` → 200/503，取决于配置已加载且至少一个可用上游
- `GET /metrics` → Prometheus 文本
- `GET /routes` → `[{id, uri, upstreams, predicates, filters}]`
- `GET /version` → `{"name":"next-web-gateway","version":"…"}`
- 其余路径 → 404。admin 不经过路由/过滤器链，不做鉴权（只监听 `127.0.0.1`，非本机绑定需显式配置并在 README 警告）。

`suppress_error_log` 改为 `false`。

### D11 [决定] 过滤器 / 谓词逐条修正

谓词：

| 谓词 | 修正 |
|---|---|
| `Path` | 改用 `req_header().uri.path()`（不含 query）；支持 `{var}` / `{*var}` 模板并捕获到 `ctx.path_variables`；`/**` 匹配零段或多段 |
| `Host` | `*.example.com` 匹配一层，`**.example.com` 匹配一层或多层，`example.com` 精确；读 `Host` 头，缺则回退 URI authority |
| `Header` | 遍历**所有**同名值（OR）；正则用完整匹配（`^(?:re)$`）；`Header=X,` 视为「头存在即可」而非恒 false |
| `Cookie` | 取第一个同名 cookie；完整匹配 |
| `Query` | 取第一个值；完整匹配 |
| `Method` | 非法 token 启动失败 |
| `RemoteAddr` | IPv4-mapped IPv6 归一；非法 CIDR 启动失败 |
| `XForwardedRemoteAddr` | 谓词参数即受信代理列表（与 SCG 一致，见 D15）：直连对端不在列表内 → 不命中；在则从 XFF 右往左跳过受信代理，取第一个不受信地址再比对 |
| `Weight` | 桶改为每请求随机（`rand` 从 workspace 引；或 `uuid` v4 的低 32 位），仍是 `start <= n < end` |
| `After`/`Before`/`Between` | value 统一 `trim`；`Between` 支持 `start/end` 或 `start,end` 两种分隔；保留时区偏移，`[Region/Zone]` 后缀先解析为偏移再用 |
| 全部 | 解析失败 → 启动错误，不再是 `Nothing` |

过滤器：

| 过滤器 | 修正 |
|---|---|
| 全部 | 参数语法统一为 SCG 的 `Name=arg1, arg2`；同时接受旧的 `Name=a:b`（`warn!` 一条弃用） |
| `AddRequestHeader(sIfNotPresent)` | 改为 SCG 名 `AddRequestHeadersIfNotPresent`（保留旧名作别名） |
| `AddRequestParameter` | 保留重复键与顺序，不再经 `HashMap`；用 `form_urlencoded::Serializer` 但保留原编码 |
| `MapRequestHeader` | 参数 `trim` |
| `PreserveHostHeader` | 按 D7 实现（真正的 host 保留） |
| `RewritePath` | `Regex::replace_all`；支持 `$1` 反向引用 |
| `RewriteResponseHeader` | 参数顺序改为 `header, regexp, replacement`；在现有值上做 `replace_all` |
| `SetPath` / `SetRequestHeader` / `AddRequestHeader` 等 | 支持路径变量替换 `${var}` 与 `#{...}`? **不做**；只支持 `{var}` 从 `ctx.path_variables` 替换 |
| `SetStatus` | 非 100–599 → 启动失败 |
| `RequestSize` | 按 D9 第 6 条改为真实累计 |
| `RequestHeaderSize` | 非法值启动失败；估算包含请求行 |
| `RequestRateLimiter` | key = 客户端 IP（可由 `X-Forwarded-For` 最右段覆盖，但只在来源受信时）；修掉重复计数（只在请求趟运行）。限速判定改用 `Rate::observe(key, 1)` 的**返回值**（当前窗口累计计数）与 `limit` 比较，而不是 `Rate::rate()`（后者返回的是**上一个已完成窗口**的估算，所以现在有一整个窗口的滞后、首窗必然放行、实际可超发近 2 倍）。429 带 `Retry-After: 1` 与 `X-RateLimit-*` |
| 路由级 `rate_limiter` | 删除（与 `RequestRateLimiter` 重复且语义不一致），改为启动时迁移成一条 `RequestRateLimiter` 过滤器并 `warn!` |
| `SecureHeaders` | 补齐 SCG 默认集（含 CSP、`X-Download-Options`、`X-Permitted-Cross-Domain-Policies`）；改为**覆盖**而非仅缺失时插入（SCG 是覆盖） |
| `LocalResponseCache` | key 加入 `Vary` 声明的头、`Cookie`、`Authorization`、`Accept-Encoding`、`Host`；单个超大条目只淘汰该条目；`no-cache` 请求不走缓存；加 `Age`；`filter()` 不再是空转发（把逻辑收进实现，去掉 `next_gateway_application` 里的旁路调用） |
| `RemoveJsonAttributesResponseBody` | 只在 `Content-Encoding` 为空或 `identity` 时处理，否则 `warn!` 跳过 |
| `SaveSession` | `ctx.session` 无来源 → 该过滤器启动时报「不支持的过滤器」（Phase 1 明确不支持，而不是静默 no-op） |
| `TokenRelay` | 同 `SaveSession`：无 OAuth2 client 时启动报不支持 |
| `CircuitBreaker` | 按 D8 实现 |
| 未知过滤器名 | 启动失败并列出可选名称 |

### D12 [决定] `application.yaml` 与示例

- 重写 `next-web-gateway/application.yaml` 为 `next.gateway` 且**能直接跑起来**的样例（含两条路由、一条 `lb://` 一条 `http://`、StripPrefix、限流）。
- 新增 `next-web-gateway/examples/config/`：`minimal.yaml`、`lb.yaml`、`https-upstream.yaml`。
- 集成测试复用 `examples/config/minimal.yaml` 的结构（测试内联生成，避免读文件顺序依赖）。

### D13 [决定] 测试策略

- **单元测试**：每个修复点一个 test，命名 `snake_case` 描述行为。
- **集成测试** `next-web-gateway/tests/`：
  - `support/upstream.rs`：`tokio::net::TcpListener` 的最小 HTTP/1.1 上游（可编程响应状态/头/体，可记录收到的请求）。
  - `support/client.rs`：最小 HTTP/1.1 客户端（`TcpStream` + 手写请求/响应解析，支持 chunked 与 `Connection: close`）。
  - `support/gateway.rs`：在随机端口启动真实 pingora 实例，返回 base URL 与 `AdminHandle`；测试结束 `shutdown`。
    已核实的可行路径：`ServerConf::default()` + `http_proxy_service_with_name(&conf, app, "test")` + `Service::add_tcp(addr)`，然后在 tokio runtime 里直接调 `Service::start_service(fds, shutdown, listeners_per_fd, ready_notifier)`（`ServiceReadyNotifier::new(watch::Sender)` 文档明确说明是给测试用的；`ShutdownWatch = watch::Receiver<bool>`，drop 掉 tx 即触发关闭；`fds` 参数在 unix 上传 `None`、Windows 上没有该参数）。宿主端口先绑 `127.0.0.1:0` 探出空闲端口再释放，避免被测代码自己选端口。
  - 用例：路由命中/未命中(404)、`StripPrefix`、`AddRequestHeader`/`SetResponseHeader`、`RewritePath`、`RequestRateLimiter`(429)、`RequestSize`(413，含 chunked)、`SetStatus`、`PreserveHostHeader`、`lb://` 双上游轮询、上游不可达(502)、停掉的 upstream(503)、`X-Forwarded-For` 追加、`X-Request-Id` 生成与透传、访问日志字段、`/metrics` 有数据、`/health/ready` 状态。
  - 不引新依赖（不引 reqwest / hyper client）。
- **TLS 上游测试**（G3 的证据）：把用 `openssl` CLI 生成的静态 PEM 夹具提交到 `next-web-gateway/tests/fixtures/tls/`（`ca.pem`、`server.pem` + `server.key`，CN/SAN = `localhost`；另加一份 `selfsigned.pem` 用于负例）。Mock upstream 用 `tokio-rustls::TlsAcceptor` 加载夹具提供 HTTPS 服务（`tokio-rustls` / `rustls` / `rustls-pemfile` 只作为 **dev-dependency**，它们已在 workspace 依赖里）。三个用例：`ca_file` 指定 CA → 200；不配 CA → 502（证书校验失败）；`insecure_skip_verify: true` → 200 且日志有 warn。
- **二进制子进程测试** `tests/cli.rs`：用 `env!("CARGO_BIN_EXE_next-web-gateway")` 起子进程，断言 `--check` 对合法配置退出 0、对非法配置退出 **2** 且 stderr 含错误路径、`--help` 退出 0、缺失配置文件退出 2。守护进程（`-d`）与优雅升级（`-u`）不自动化，README 里标注为手动验证项。
- **配置测试**：`tests/config_validation.rs`，每个「启动失败」规则一个 use case，断言错误消息包含路径。

### D14 [决定] 打包与文档

- `next-web-gateway/Dockerfile`：多阶段（`rust:1.98-slim` builder + `debian:bookworm-slim` runtime），`EXPOSE 8080 9001`，`ENTRYPOINT ["/usr/local/bin/next-web-gateway"]`。
- `next-web-gateway/README.md`：概览 → 快速开始（docker 与本地）→ 配置参考（逐字段表）→ 谓词表 → 过滤器表 → 运维（健康检查/指标/日志/退出码）→ 与 SCG 的差异清单 → Phase 2/3 路线。
- 根 `README.md` / `README.zh-CN.md` 加一节指向 gateway README（保持既有风格，简短）。

### D15 [决定] 受信代理（trusted proxies）

新增顶层 `next.gateway.trusted_proxies: [CIDR]`，默认 `[]`。

- **默认 `[]` = 谁都不信。** 于是：
  - `RequestRateLimiter` 的限流 key 取**直连对端地址**；只有 `trusted_proxies` 非空且直连对端命中时，才从 `X-Forwarded-For` 里**从右往左**跳过受信代理，取第一个不受信的地址作为 key。
  - `XForwardedRemoteAddr` 谓词的受信列表**用谓词自己的参数**（与 SCG 一致：该谓词的参数就是 trusted proxies），语义改为「先确认直连对端在该列表内；是则从 XFF 右往左跳过受信代理取第一个不受信地址；否则直接看不命中」。谓词参数非法 → 启动失败。
- `X-Forwarded-*` 的**追加**行为不受此配置影响（永远追加，不覆盖客户端已有值）。

### D16 [决定] 熔断器配置位置

新增顶层 `next.gateway.breakers: { <id>: { enabled, failure_threshold, wait_duration_in_open_state, half_open_permits } }`，默认 `failure_threshold: 10`、`wait_duration_in_open_state: 5s`、`half_open_permits: 1`、`enabled: true`。

- 熔断器**按 id 全局共享**（多条路由可引用同一个），与现有行为一致，也是 Phase 2 fallback 的载体。
- 同时接受旧格式 `gateway.circuitbreaker.<id>`（camelCase 键 `failureThreshold` / `waitDurationInOpenState`），加载时 `warn!` 一条弃用提示；新旧同时出现 → 配置错误。
- `CircuitBreaker=<id>` 引用了**未定义**的 id → **启动失败**（A1），错误信息列出已定义的 id。

### D17 [决定] 过滤器执行模型：阶段声明 + YAML 顺序

这是对现状最关键的结构性修正——现状把「过滤器跑哪一趟」交给每个过滤器自己 `if request_header().is_some()` 判断，导致 `RequestRateLimiter` 在请求与响应两趟各跑一次。

- `pub trait GatewayFilter` 增加 `fn phase(&self) -> FilterPhase`，`pub enum FilterPhase { RequestHeader, ResponseHeader, ResponseBody }`。
- 过滤器链在每一趟只执行 `phase()` 与当前趟相同的过滤器，其余跳过。**顺序 = YAML 声明顺序**（与 SCG 的路由级过滤器一致），不做固定重排。
- 网关自产响应（404 / 429 / 503 / 502 / 504 / `RedirectTo`）：
  - **请求侧过滤器不再执行**（没有上游请求可改；已经执行过的那几个不回滚）。
  - **响应侧两趟都执行**（`ResponseHeader` → 再按需 `ResponseBody`），保证 `AddResponseHeader` / `SecureHeaders` / `RemoveResponseHeader` / JSON 改写对错误响应同样生效。
- 请求体过滤器（`RequestSize` 的真实累计计数、Phase 2 的 `ModifyRequestBody`）走 `ProxyHttp::request_body_filter`，`FilterPhase` 增加 `RequestBody`，`phase()` 只对它返回该值。

---

## 7. 当前基线（Phase 1 起点，已实测）

| 指标 | 实测值 | 命令 |
|---|---|---|
| 编译 | 通过，13 条 warning | `cargo check -p next-web-gateway --all-targets` |
| 单元测试 | 37 passed | `cargo test -p next-web-gateway --lib` |
| doctest | 8 failed（引用不存在的 crate `circuit_breaker::CircuitBreaker`） | `cargo test -p next-web-gateway --doc` |
| clippy | 64 warning | `cargo clippy -p next-web-gateway --all-targets` |
| 集成测试 | 无 `tests/` 目录 | — |
| 可执行文件 | 无 | — |

Phase 1 的验收线：以上全部收敛为「编译 0 warning、clippy `-D warnings` 0、`cargo test` 全绿（含 doctest 与集成测试）」，并且 `next-web-gateway --check` 能校验仓库自带的 `application.yaml`。

## 8. 留给实现者

以下是实现者可以自己决定、不需要回来的：

- 具体模块文件拆分粒度（建议：`config/` 下按 `server|upstream|route|predicate|filter|error|logging` 分文件；`upstream/` 下分 `resolver|load_balancer|dns_cache`；`admin/` 独立；`metrics/` 独立）。
- 错误类型的具体变体划分（建议 `GatewayConfigError` + `GatewayRuntimeError` 两个 `thiserror` 类型）。
- 指标 histogram 的桶边界实现细节（固定数组即可）。
- 集成测试 harness 的内部结构（只要 `support/` 三件套的对外签名稳定）。
- YAML → 内部结构的中间表示是否单独一层（建议有，便于校验错误带路径）。
- 日志字段的具体格式化实现（`tracing` 的 `field::display` 即可）。
- 是否保留 `RouteWork` 枚举（建议保留，`lb` 分支改用它而不是 `contains('.')`）。

## 9. Phase 2 / Phase 3 路线图（不在本次计划内）

**Phase 2（SCG 特性对齐）**
- 全局过滤器层：`RemoveHopByHopHeaders`、`XForwardedHeaders`、`ForwardedHeaders`、`RouteToRequestUrl`、`GatewayMetrics`、`WebsocketRouting`（若 pingora 支持）
- `Retry` 过滤器（次数、状态码、方法白名单、退避、`retry-after`）
- `ModifyRequestBody` / `ModifyResponseBody`（需要请求体缓冲 + 流式上限）
- `CacheRequestBody`、`FallbackHeaders`、`CircuitBreakerFallback`
- CORS（`global_cors` + 路由 `metadata.cors`，含预检）
- actuator：`/actuator/gateway/routes`、`/actuator/gateway/globalfilters`、`/actuator/health/{component}`
- 熔断升级为 failure-rate + 滑动窗口 + 半开许可 + 慢调用
- 分布式追踪（`tracing-opentelemetry`）+ 指标推送
- 上游 HTTP/2（`ALPN::H2`）

**Phase 3（运维演进）**
- 配置热重载（`arc-swap` + 文件 watcher + `POST /admin/reload`）
- 网关侧 TLS 终止（`add_tls` + 证书热加载）
- 服务发现（`UpstreamResolver` 的 Nacos / Consul / K8s 实现）
- mTLS 上游、连接池调参、`bind_to`、DNS 主动刷新
- WebSocket / gRPC 透传
