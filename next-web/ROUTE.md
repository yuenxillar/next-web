# Next Web 路线图

> - 当前阶段：`0.x`，API 仍在演进
> - 最后更新：2026-09-27
> - 目标：在保留“宏驱动 + 统一应用模型”特色的前提下，把 Next Web 从“能用”推进到“可长期维护、可用于生产”。

## 0. 结论摘要

核心能力已经成型：应用启动、生命周期、事件、自动配置、依赖注入/单例、路由宏、配置绑定、消息源、调度、WebSocket、OpenAPI，以及 Redis、数据库、MQ/MQTT、邮件等集成都有实现或接入点。

当前最主要的缺口不是功能数量，而是下面这几件事没有同步收口：

- 没有 CI 质量门禁，`fmt`、`clippy`、全量测试、文档构建和依赖检查还没有形成固定基线。
- 当前工作区存在失败的 `next-web-core` 单元测试（CORS、Cookie 相关），需要先清零。
- 公开路径中仍有 `todo!()` / `unimplemented!()`，其中 `next-web-security` 约 64 处，必须先决定“补全”还是“明确标记不支持并返回结构化错误”。
- 版本、MSRV、Feature 组合和发布流程还没有统一策略；例如 `next-web` 包版本与 workspace 依赖声明版本需要对齐。
- MessageSource 已有回退实现，定时任务已有调度核心，但缺端到端测试、生产化能力与文档化说明。

建议顺序：先做 P0 质量基线，再补齐应用模型与消息/配置，随后推进调度、安全和数据集成，最后在 1.0 前冻结 API。

## 1. 原始关注点

这两项继续保留为主线任务：

- **messageSource 是否有回退机制**：已有实现，归入 M2，需要明确回退顺序、补端到端测试并写入文档。
- **定时任务**：已有 `#[scheduled]`、`ScheduledTask`、`JobSchedulerManager` 和内存仓库，归入 M3，重点补持久化、集群和治理能力。

## 2. 现状盘点

| 能力域 | 当前状态 | 主要缺口 |
| --- | --- | --- |
| 应用启动、生命周期、事件 | 基本完成 | 启动失败诊断、优雅关闭、生命周期顺序测试 |
| DI / 单例 / Provider | 基本完成 | 异步 Provider、异步 eager creation、循环依赖诊断 |
| 配置 / Environment / Binder | 基本完成 | Profile 优先级、错误信息、`#{}` 文档与测试、配置来源说明 |
| 自动配置 / 条件装配 | 基本完成 | 条件诊断、Feature 组合测试、starter 约定 |
| HTTP / 路由 / 提取器 | 基本完成 | PathContainer 实现、过滤器链、路由匹配边界测试 |
| 消息源 / i18n | 基本完成 | 回退链端到端测试、LocaleResolver 扩展、复数与格式化规则 |
| 调度 | 核心可用 | 持久化仓库、集群抢占、misfire 策略、动态管理、指标 |
| 安全 | 部分实现 | 范围收敛、约 64 处 `todo!()`、OIDC/会话/CSRF 等能力 |
| Redis / 数据库 | 可用 | 集群/Redlock、事务传播、迁移、容器化集成测试 |
| MQ / MQTT / Mail / 微信 / 短信 / 支付 | 可用 | 统一适配层、重试/死信、可观测性、连接恢复测试 |
| Actuator / 指标 / 日志 | 部分完成 | 端点补齐、健康贡献者、Trace 传播、生产默认值 |
| 文档 / 示例 / 发布 | 部分完成 | 每 crate 文档、Cookbook、迁移指南、发布自动化 |

## 3. 缺口清单

优先级定义：

- **P0**：影响正确性、稳定性、安全性或后续开发效率，应在下一个 minor 之前完成。
- **P1**：生产可用性所需，决定框架能否用于长期项目。
- **P2**：生态、体验和打磨项，可以在核心稳定后推进。

### 3.1 P0：质量基线与工程化

- [ ] 建立 CI，至少覆盖 `cargo fmt --check`、`cargo clippy --workspace --all-targets --all-features`、`cargo test`、`cargo doc` 和 `cargo deny check`。
- [ ] 清零当前失败测试：`next-web-core` 的 `cors` 与 `http::cookie` 相关测试。
- [ ] 审计公开路径中的 `todo!()` / `unimplemented!()`，每一处要么补实现，要么改成明确的 `Unsupported` 错误并写清楚适用范围。
- [ ] 为 workspace 建立 Feature 组合矩阵：默认、`full`、单 Feature、核心最小集，至少各跑一组测试。
- [ ] 统一版本策略：让包版本、workspace 依赖版本、CHANGELOG 和发布流程保持一致。
- [ ] 明确 MSRV、edition 和 breaking change 策略，并写入贡献文档。

### 3.2 P0：核心应用模型

- [ ] 实现异步 Provider 与异步 eager creation，替换当前 “not supported yet” 的 panic 路径。
- [ ] 完成 `next-web-core` 中的公开占位实现：`PathContainer`、`HttpFilterChainShare` 等。
- [ ] 梳理 `ApplicationContext`、Singleton、Provider 的生命周期顺序，补循环依赖与缺失依赖的诊断信息。
- [ ] 统一错误类型：公开 API 尽量返回结构化错误，不再依赖 panic 作为正常失败路径。

### 3.3 P0：配置、消息与上下文

- [ ] 明确并测试 messageSource 回退顺序：请求 Locale → default Locale → system Locale → base bundle → common messages → parent message source → `use-code-as-default-message`。
- [ ] 增加 `fallback-to-system-locale` 的端到端测试，并补齐自动配置场景测试。
- [ ] 为 `${property:default}` 与 `#{ENV_VAR:default}` 补文档、示例和失败信息断言。
- [ ] 梳理 `next.config.location`、`next.config.additional-location`、`next.config.import`、Profile 与命令行参数的优先级，并写成可测试的规范。
- [ ] 为配置绑定错误补充属性 key、目标字段路径、候选名称和修复建议。

### 3.4 P1：调度与任务

- [ ] 提供持久化 `ScheduledJobRepository` 参考实现（数据库/Redis），验证重启后恢复调度状态。
- [ ] 定义多实例语义：任务抢占、锁、实例分配或明确的 single-instance 模式。
- [ ] 增加 misfire 策略：错过执行时间是补跑、跳过，还是按策略配置。
- [ ] 支持运行时暂停、恢复、手动触发和重载任务。
- [ ] 为调度补指标与日志：执行次数、失败次数、耗时、下次执行时间、最近错误。
- [ ] 补充时区、夏令时、fixed-rate、cron 边界和长时间阻塞任务的测试。
- [ ] 明确任务失败后的重试/死信策略，并与 `next-web-retry` 的关系写清楚。

### 3.5 P1：安全

安全模块是当前最大的不确定性，应先定义边界，再决定实现顺序。

- [ ] 明确支持范围：Spring Security 子集、Shiro 子集，还是两者都作为可选兼容层。
- [ ] 清理 `next-web-security` 中的约 64 处 `todo!()`：补实现，或改为明确的 `Unsupported` 错误。
- [ ] 补齐认证/授权基础链路：认证过滤器、授权决策、权限合并、拒绝响应。
- [ ] 补齐会话相关能力：会话创建、Session ID 轮换、Remember-Me、并发会话控制。
- [ ] 完成 CSRF、Headers、CORS 等 Web 安全配置器。
- [ ] 评估并补齐 OIDC Back-Channel Logout、LDAP、PAM 等高级能力。
- [ ] 增加安全集成测试：登录、退出、权限拒绝、CSRF、OIDC、会话固定攻击防护。

### 3.6 P1：数据、消息与集成

- [ ] 数据库：补 MySQL/PostgreSQL 的事务传播、连接池配置、迁移与容器化集成测试。
- [ ] Redis：补 Cluster、Redlock、锁续约与异常恢复测试，明确 `LockGuard::unlock` 的使用约束。
- [ ] MQ / MQTT：统一连接生命周期、重连、重试、死信、序列化与可观测性。
- [ ] 邮件、微信、短信、支付等集成：补真实失败场景、限流、幂等和重试策略。
- [ ] 所有外部依赖集成都提供可本地启动的集成测试方案（Testcontainers 或等价方案）。

### 3.7 P2：可观测性与生产就绪

- [ ] 补齐 Actuator 端点：health、info、metrics、env、thread、system、config props。
- [ ] 统一 Trace / Metrics / Log 的关联方式，确保跨异步任务与后台任务可追踪。
- [ ] 完成优雅关闭：HTTP、调度、后台服务、连接池和消息消费者的关闭顺序。
- [ ] 增加压测与基准：路由、绑定、消息源、调度、序列化。
- [ ] 补充内存、线程池、连接池和限流相关的生产默认值。

### 3.8 P1/P2：文档、示例与开发者体验

- [ ] 每个 crate 的 README、API 文档和最小示例与当前实现对齐。
- [ ] 写一份 Cookbook：配置、DI、路由、校验、安全、调度、国际化、数据访问、可观测性。
- [ ] 写迁移指南：0.x 内部 breaking change 和未来 1.0 迁移路径。
- [ ] 为每个 feature 提供可运行的示例，并让 examples 进入 CI。
- [ ] 评估项目生成器/CLI，降低新项目起步成本。
- [ ] 统一错误信息风格，尤其是启动失败、配置失败和依赖缺失场景。

### 3.9 P1：发布与生态治理

- [ ] 建立发布流程：版本号、CHANGELOG、crates.io 发布顺序、Git tag。
- [ ] 加入 `cargo semver-checks` 或等价工具，在 1.0 前逐步减少意外 breaking change。
- [ ] 增加依赖与许可审计：`cargo deny`、安全公告、重复依赖。
- [ ] 明确各 crate 的维护状态：active、experimental、deprecated。

## 4. 里程碑

版本号只是建议，可在发布时调整。

### M1：0.3 — 质量基线与核心补全

**目标**：让仓库处于“每次提交都可验证”的状态。

**交付**

- CI、fmt、clippy、test、doc、deny 全部跑通。
- 失败测试清零。
- 公开路径中的 `todo!()` / `unimplemented!()` 完成分类：实现或明确不支持。
- 版本、MSRV、Feature 策略形成文档。
- 异步 Provider 与核心占位实现进入开发。

**退出标准**

- 默认分支 CI 全绿。
- 不再有“无说明、无边界”的 panic/todo 出现在公开 API。
- 新贡献者能根据文档本地复现完整验证流程。

### M2：0.4 — 配置、消息与应用模型完善

**目标**：把应用模型的核心语义固定下来。

**交付**

- messageSource 回退链端到端测试与文档。
- 配置来源、Profile、占位符优先级规范。
- 异步 Provider / eager creation 完成。
- 应用上下文生命周期与错误诊断完善。
- 配置绑定错误信息达到“能直接定位问题”的标准。

**退出标准**

- 新项目可以通过配置、DI、消息源、国际化和错误处理完成一个完整业务闭环。
- 所有公开配置项都有文档和默认值说明。

### M3：0.5 — 调度与安全可用

**目标**：让框架具备生产环境中常见的后台任务与安全基础能力。

**交付**

- 持久化调度仓库参考实现。
- 多实例调度语义、misfire 策略、动态管理、调度指标。
- 安全模块边界确定，核心认证/授权/会话/CSRF 链路可用。
- 安全集成测试与安全默认配置。

**退出标准**

- 调度任务能在重启后恢复，并能在多实例下明确行为。
- 默认安全配置不会给出错误的安全感，未实现能力有明确报错。

### M4：0.6 — 数据、消息与生产可观测性

**目标**：覆盖典型生产应用的外部依赖与运行观测需求。

**交付**

- 数据库事务、连接池、迁移与集成测试。
- Redis 集群/Redlock 与锁恢复测试。
- MQ/MQTT 统一生命周期与可观测性。
- Actuator、指标、Trace、优雅关闭完善。
- 示例与 Cookbook 覆盖主要集成。

**退出标准**

- 常见生产部署形态（单实例、多实例、容器化、外部依赖不可用）有明确行为。
- 能从日志、指标和健康检查定位一次真实故障。

### M5：1.0 — API 稳定与生态

**目标**：冻结公共 API，建立长期兼容承诺。

**交付**

- 公共 API 审查与 `semver-checks`。
- MSRV、Feature 和弃用策略正式化。
- 发布自动化、CHANGELOG 自动化、文档站点。
- 核心 crate 达到稳定维护状态。

**退出标准**

- 1.0 之后新增能力以兼容方式演进。
- 破坏性变更必须有迁移指南和明确的 major 版本窗口。

## 5. 单项工作的完成标准

每个进入路线图的功能，完成时必须满足：

- [ ] 有明确的适用边界和不支持场景。
- [ ] 有单元测试；涉及外部依赖时，有集成测试或可复现的测试方案。
- [ ] 有文档：配置项、默认值、示例和错误行为。
- [ ] 有示例或测试证明真实调用链可用。
- [ ] 不在公开 API 中新增未说明的 `panic!`、`todo!()`、`unimplemented!()`。
- [ ] 更新 CHANGELOG，并说明是否 breaking。

## 6. 风险与依赖

| 风险 | 影响 | 应对 |
| --- | --- | --- |
| 安全模块移植范围过大 | 长期拖慢核心进度 | 先定义支持子集，未实现能力明确报错 |
| API 频繁调整 | 用户迁移成本高 | 0.x 阶段集中调整，1.0 前冻结 |
| Feature 组合过多 | CI 时间不可控 | 矩阵 + 抽样组合，核心组合必跑 |
| 集成测试依赖外部服务 | 本地与 CI 不稳定 | 容器化、可选 Feature、超时与重试 |
| MSRV 偏高 | 限制用户升级范围 | 明确是否为硬要求，必要时调整 |
| 调度多实例语义不清 | 生产重复执行或漏执行 | 在 M3 前确定 single-instance / claim / lock 模型 |

## 7. 跟踪建议

建议为路线图建立统一标签：

- 优先级：`P0`、`P1`、`P2`
- 领域：`area/core`、`area/config`、`area/context`、`area/scheduling`、`area/security`、`area/data`、`area/observability`、`area/docs`、`area/ci`
- 类型：`type/bug`、`type/feature`、`type/docs`、`type/chore`
- 状态：`status/blocked`、`status/ready`、`status/in-progress`

路线图不是一次性文档：每次 minor 发布前更新一次“已完成 / 延期 / 取消 / 新增”，并保留变更原因。
