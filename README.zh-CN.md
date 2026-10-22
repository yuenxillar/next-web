[English](README.md) | 简体中文

# Next Web

> Build web applications with joy and simplicity.

[![Crates.io](https://img.shields.io/crates/v/next-web.svg)](https://crates.io/crates/next-web)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#许可证)

Next Web 是一个基于 Tokio 与 Axum 的 Rust Web 应用框架。它借鉴 Spring 生态中成熟的应用组织方式，通过宏、自动配置、依赖注入、配置属性、生命周期、事件与定时任务等能力，把 Web 服务常用的基础设施收拢到统一的应用模型中。

它想解决的不只是“如何写一个路由”，而是“如何组织一个会持续生长的 Rust Web 应用”。你可以从一个很小的 HTTP 服务开始，需要时再逐步加入配置绑定、单例管理、安全、国际化、数据访问、消息队列、API 文档与可观测性等模块。

## 核心能力

- **宏驱动开发**：路由、依赖注入、配置属性、定时任务、重试、参数校验等通过过程宏声明，让代码更接近业务本身。
- **应用上下文与单例管理**：`ApplicationContext` 统一管理应用组件，`#[singleton]` 与 `FindSingleton` 让依赖关系清晰可见。
- **配置与环境变量**：支持 YAML / properties 配置文件、Profile、配置属性绑定，以及 `${property:default}` 与 `#{ENV_VAR:default}` 占位符。
- **自动配置与条件装配**：通过 `AutoConfiguration` 和 `#[auto_configuration]` 组织 starter 与应用级配置。
- **模块化扩展**：核心、上下文、宏和各类集成分别位于独立 crate，可按需启用 feature，按需引入能力。
- **面向真实服务**：提供或集成 Redis、数据库、MQTT、消息队列、邮件、WebSocket、OpenAPI、安全、定时任务、XXL-JOB 等常见组件。
- **基于成熟生态**：运行时建立在 Tokio 与 Axum 之上，Windows 默认使用 mimalloc，Linux 默认使用 jemalloc。

## 快速开始

### 环境要求

- Rust 1.98.1 或更高版本
- Cargo

### 添加依赖

```toml
[dependencies]
next-web = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

如果需要一次启用大部分可选能力，可以使用 `full` feature：

```toml
[dependencies]
next-web = { version = "0.2", features = ["full"] }
```

### 最小示例

```rust
use next_web::{
    Application, NextWebApplication,
    macros::bind::get_mapping,
};

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[get_mapping(path = "/hello")]
async fn hello() -> &'static str {
    "Hello World!"
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}

```

运行：

```bash
cargo run
```

默认端口为 `11000`。可以通过 `next.server.address` 和 `next.server.port` 调整监听地址。

## 配置与环境变量

框架默认会加载 `application.yaml`、`application.yml` 或 `application.properties`，并支持通过 `next.config.location`、`next.config.additional_location` 与 `next.config.import` 调整配置来源。例如：

```yaml
next:
  server:
    address: 0.0.0.0
    port: 11000
```

配置值支持两类占位符：

- `${property:default}`：读取配置属性，例如 `${next.application.name:demo}`。
- `#{ENV_VAR:default}`：读取进程环境变量，例如 `#{DATABASE_URL:jdbc:mysql://localhost:3306/default}`。冒号后是默认值；没有默认值且环境变量不存在时，配置绑定会返回明确错误。

在 YAML 中，以 `#` 开头的值需要加引号，例如：

```yaml
next:
  datasource:
    url: "#{DATABASE_URL:jdbc:mysql://localhost:3306/default}"
```

## 功能模块

项目以 workspace 组织，框架自身与各类集成分开维护，便于按需组合。主要 crate 如下：

| Crate | 说明 |
| --- | --- |
| `next-web` | 应用入口、Web 服务器、配置加载、生命周期、事件、自动配置与诊断 |
| `next-web-core` | 核心抽象：环境与配置绑定、HTTP 抽象、过滤器、指标、消息与通用工具 |
| `next-web-context` | `ApplicationContext`、单例、`MessageSource`、属性源与占位符支持 |
| `next-web-singletons` | 单例注册、创建与依赖解析 |
| `next-web-macros` | 路由、依赖注入、配置属性、定时任务、重试、脱敏等过程宏 |
| `next-web-security` | 安全与认证授权扩展 |
| `next-web-websocket` | WebSocket 服务端支持 |
| `next-web-open-api` | 基于 utoipa 的 OpenAPI / Swagger 文档能力 |
| `next-web-data-redis` | Redis 集成，可选分布式锁 |
| `next-web-data-database` | MySQL / PostgreSQL 数据访问与事务支持 |
| `next-web-mqtt` / `next-web-mq-*` | MQTT 与 Kafka、RabbitMQ、RocketMQ 等消息集成 |
| `next-web-mail` | 邮件发送与 MIME 消息构建 |
| `next-web-xxl-job` | XXL-JOB 执行器集成 |
| `next-web-state-machine` | 状态机建模与运行支持 |
| `next-web-retry` | 重试模板与 `#[retryable]` |
| `next-web-actuator` | 健康检查、环境信息、线程与系统指标 |
| `next-web-gateway` | 基于 Pingora 的网关能力 |
| `next-web-ai` | AI / DeepSeek 等模型调用集成 |
| `next-web-document` | Excel、CSV、PDF 等文档解析与生成 |
| `next-web-pay-*` / `next-web-wechat` / `next-web-sms` | 支付、微信与短信等业务集成 |

## Feature 标志

`next-web` 默认启用 `decrypt-properties`，其他能力按需开启：

| Feature | 说明 |
| --- | --- |
| `full` | 启用常用可选能力的组合 |
| `tls-rustls` | 基于 rustls 的 TLS 支持 |
| `enable-web-security` | 启用安全模块 |
| `enable-websocket` | 启用 WebSocket |
| `enable-scheduling` | 启用定时任务 |
| `enable-retry` | 启用重试能力 |
| `enable-i18n` | 启用国际化 |
| `enable-open-api` | 启用 OpenAPI / Swagger 文档 |
| `enable-prometheus` | 启用 Prometheus 指标 |
| `enable-state-machine` | 启用状态机 |
| `enable-thread-pool` | 启用线程池能力 |
| `embed-resources` | 将资源嵌入二进制 |
| `embed-resources-debug` | 调试构建也嵌入资源 |
| `digester` | 启用摘要算法相关能力 |
| `decrypt-properties` | 启用配置解密（默认启用） |
| `trace-log` | 追踪特殊的日志 |

## 运行示例

`examples` 目录包含一组可直接运行的示例：

| 示例 | 命令 | 说明 |
| --- | --- | --- |
| 基础应用 | `cargo run -p next-web-examples --bin application` | 路由、单例与基础应用结构 |
| 配置属性 | `cargo run -p next-web-examples --bin properties` | `#[configuration_properties]` 与配置绑定 |
| 自动配置 | `cargo run -p next-web-examples --bin auto_configuration` | `AutoConfiguration` 与条件装配 |
| 应用事件 | `cargo run -p next-web-examples --bin application_event` | 应用事件监听与发布 |
| 生命周期 | `cargo run -p next-web-examples --bin application_lifecycle` | 应用启动与关闭生命周期 |
| 定时任务 | `cargo run -p next-web-examples --bin scheduled` | `#[scheduled]` 与任务注册 |
| 参数解码与校验 | `cargo run -p next-web-examples --bin decoder_and_vaild` | `Data`、`Validated` 与校验器 |
| 国际化 | `cargo run -p next-web-examples --bin i18n` | `MessageSource` 与多语言消息 |
| 安全 | `cargo run -p next-web-examples --bin security` | 安全模块接入 |
| WebSocket | `cargo run -p next-web-examples --bin ws` | WebSocket 处理器注册 |
| Redis | `cargo run -p next-web-examples --bin redis` | Redis 模板与监听 |
| 数据库 | `cargo run -p next-web-examples --bin database` | 数据库服务与事务 |
| 重试 | `cargo run -p next-web-examples --bin retry` | 重试模板与 `#[retryable]` |
| 状态机 | `cargo run -p next-web-examples --bin state_machine` | 状态机配置与运行 |
| OpenAPI | `cargo run -p next-web-examples --bin open_api` | API 文档生成 |

更多示例（邮件、MQTT、XXL-JOB、支付、AI、流式响应等）见 `examples/` 目录。

## 网关

`next-web-gateway` 是基于 pingora 的 Spring Cloud Gateway 对应物：路由、谓词与过滤器全部用 YAML
声明，配有会一次性报出全部问题的配置校验、静态负载均衡、熔断、限流、进程内响应缓存、结构化访问
日志与 Prometheus 指标。

```bash
cargo run -p next-web-gateway -- --check --config next-web-gateway/application.yaml
cargo run -p next-web-gateway -- --config next-web-gateway/application.yaml
```

配置项、谓词与过滤器清单、admin 端点以及与 Spring Cloud Gateway 的差异见
[`next-web-gateway/README.md`](next-web-gateway/README.md)。

## 版本与反馈

当前版本处于 `0.x` 阶段，API 仍在持续演进。欢迎通过 [Issues](https://github.com/yuenxillar/next-web/issues) 反馈问题、建议使用场景或参与贡献。

## 许可证

本项目采用 MIT OR Apache-2.0 双许可证，详见 [LICENSE-MIT](LICENSE-MIT) 与 [LICENSE-APACHE](LICENSE-APACHE)。
