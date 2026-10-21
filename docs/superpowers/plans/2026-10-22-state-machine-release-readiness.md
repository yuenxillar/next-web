# State Machine 发布就绪（文档 + 示例 + 版本）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 `next-web-state-machine` 具备对外发布的最低文档与示例门槛：4 个可运行示例、crate README、CHANGELOG（含迁移说明）、`0.1.0 → 0.2.0` 版本提升、死文件清理，全部以可执行命令验证。

**Architecture:** 示例放在 workspace 的 `examples/` crate（它已依赖 `next-web`（`full` feature）与 `next-web-state-machine`，每个示例是一个 `[[bin]]`，二进制内用 `assert!`/`expect` 自校验，因此 `cargo run --example <name>` 本身就是测试）。README 的快速上手片段直接取自 `examples/state_machine_basic.rs`，避免文档与代码漂移。CHANGELOG 逐个列出本次周期已合并的破坏性 API 变更；版本只提升状态机 crate，umbrella 不变。

**Tech Stack:** Rust edition 2024、tokio（`rt-multi-thread`、`macros`、`sync`、`time`）、cargo `--example`、`next-web` umbrella crate 的 `enable-state-machine` 特性。

**Spec:** 无独立 spec 文档；需求来自本会话的生产就绪评估，逐条对应"第五节 测试盲区"之外的发布门槛：README、示例、CHANGELOG/迁移、版本、死文件。

## Global Constraints

- `unsafe_code = "forbid"`（workspace lint），不得出现 `unsafe`。
- 所有 `pub` 项必须有英文文档注释；`cargo doc` 必须 0 告警。
- 生产路径禁止 `unwrap`/`panic`；**示例**允许 `expect`，但每个 `expect` 必须带一句说明（示例即文档）。
- 每个任务结束时必须：`cargo test -p next-web-state-machine` 全绿、`cargo clippy -p next-web-state-machine --all-targets` 0 告警。
- 构建统一使用 `CARGO_TARGET_DIR=D:\DevProject\Rust\next-web\.cargo-target`。
- 提交信息用 `type(scope): subject` 格式，英文正文，一次任务一次提交。
- 示例二进制名即文件名去掉 `.rs`（`basic.rs` → `basic`）。
- 示例一律 `#[tokio::main] async fn main()`，失败即 panic（`expect` 已有说明文案）。
- 每个示例文件顶部必须有一句注释：直接建机只用于演示，生产建议用 `StateMachineService`。

## Review Focus

- **单特性构建**：示例只在 `examples` crate 的 `full` 特性下编译；`next-web` 关闭 `enable-state-machine` 时不得因示例或 README 引用而失败。
- **README 与示例漂移**：README 的快速上手代码块必须与 `examples/state_machine_basic.rs` 可运行版本一致（改一处必须改另一处）。
- **版本一致性**：`next-web-state-machine` 提升到 `0.2.0` 后，`[workspace.dependencies]` 里该条目目前**没有** `version` 字段（与其它 crate 不同），发布前需要补，否则 crates.io 依赖解析会失败。
- **自定义 `Region` 实现者**：`Region::event_lock` 是新增的必需方法，CHANGELOG 必须点明，否则第三方实现编译失败且不知原因。
- **示例对 API 的过度依赖**：示例若大量使用内部类型（如 `BaseStateMachineFactory`），用户会误以为这是推荐入口；每个示例顶部需要一句"示例用工厂直接建机，生产建议用 `StateMachineService`"的注释。

---

### Task 1: 基础示例 `examples/state_machine_basic.rs`

**Files:**
- Create: `examples/state_machine_basic.rs`
- Modify: `examples/Cargo.toml`（在 `[[bin]] name = "state_machine"` 之后追加一条）

**Interfaces:**
- Consumes: `next_web::state_machine::{StateMachine, region::Region, config::…}`、
  `next_web_core::messaging::generic_message::GenericMessage…`、
  `BaseStateMachineFactory::<i32, i32>::new(Arc<dyn StateMachineModel<S,E>>, Option<…>) -> Arc<Self>`、
  `StateMachineFactory::get_state_machine(&self) -> Result<Arc<dyn StateMachine<S, E>>, BoxError>`、
  `Region::start/stop/send_event/state`、
  `ConfigurationData::<i32, i32>::with_parameters(auto_start: bool, ensemble: Option<…>, listeners: Vec<…>, security_enabled: bool, transition_decision: Option<…>, event_decision: Option<…>, event_rule: Option<…>, transition_rule: Option<…>, verifier_enabled: bool, verifier: Option<…>, machine_id: Option<String>, monitor: Option<…>, interceptors: Vec<…>)`、
  `DefaultStateMachineModel::new(configuration, Some(Vec<StateData>.into()), Some(TransitionsData::new(vec![TransitionData::new(1, 2, 100)])))`。
- Produces: 二进制 `state_machine_basic`；README 快速上手片段将复制本文件主体。

- [ ] **Step 1: 在 `examples/Cargo.toml` 注册二进制**

在 `[[bin]] name = "state_machine"` 条目之后追加：

```toml
[[bin]]
name = "state_machine_basic"
path = "state_machine_basic.rs"
```

- [ ] **Step 2: 运行以确认失败**

Run: `cargo run -p next-web-examples --bin state_machine_basic`（带 `CARGO_TARGET_DIR`）
Expected: 失败，`can't find bin 'basic'` 或 `couldn't read examples/state_machine_basic.rs`。

- [ ] **Step 3: 写示例 `examples/state_machine_basic.rs`**

要求：3 个状态 `1 → 2 → 3`，`100` 触发 `1 → 2`，`200` 触发 `2 → 3`；每步 `assert_eq!(machine.state().map(|s| *s.id()), Some(n), "…")`；结尾 `println!` 输出 `state: 3`；`main` 返回 `()`，所有 `Result` 用 `expect("…")`。

- [ ] **Step 4: 运行以确认通过**

Run: `cargo run -p next-web-examples --bin state_machine_basic`
Expected: 输出包含 `state: 3`，退出码 0。

- [ ] **Step 5: 提交**

```bash
git add examples/Cargo.toml examples/state_machine_basic.rs
git commit -m "docs(examples): add the basic state machine example"
```

---

### Task 2: 层级示例 `examples/state_machine_hierarchy.rs`

**Files:**
- Create: `examples/state_machine_hierarchy.rs`
- Modify: `examples/Cargo.toml`

**Interfaces:**
- Consumes: Task 1 的全部构造方式；`StateData::<i32, i32>::with_submachine_factory(Arc<dyn StateMachineFactory<S, E>>)`（父状态持有子机工厂）、`TransitionData::new(21, 22, 200)`（子机内部转移）。
- Produces: 二进制 `state_machine_hierarchy`。

- [ ] **Step 1: 注册二进制**

```toml
[[bin]]
name = "state_machine_hierarchy"
path = "state_machine_hierarchy.rs"
```

- [ ] **Step 2: 运行以确认失败**

Run: `cargo run -p next-web-examples --bin state_machine_hierarchy`
Expected: 失败，缺少 `examples/state_machine_hierarchy.rs`。

- [ ] **Step 3: 写示例**

结构：父机状态 `{1, 2}`，`2` 持有子机（子机状态 `{21, 22}`，`21` 初始，`200` 触发 `21 → 22`）；父机 `1 → 2` 由 `100` 触发。流程：`start` → `send_event(100)`（断言状态 `2`）→ `send_event(200)`（断言返回 `ResultType::Accepted`，这一步证明事件被转发到子机）→ 打印 `child moved: true`。

- [ ] **Step 4: 运行以确认通过**

Run: `cargo run -p next-web-examples --bin state_machine_hierarchy`
Expected: 输出 `child moved: true`（或断言通过的等价输出），退出码 0。

- [ ] **Step 5: 提交**

```bash
git add examples/Cargo.toml examples/state_machine_hierarchy.rs
git commit -m "docs(examples): add the hierarchy example"
```

---

### Task 3: 服务与持久化示例 `examples/state_machine_service.rs`

**Files:**
- Create: `examples/state_machine_service.rs`
- Modify: `examples/Cargo.toml`

**Interfaces:**
- Consumes: `next_web_state_machine::service::{state_machine_service::StateMachineService, default_state_machine_service::DefaultStateMachineService}`（`new(Arc<dyn StateMachineFactory<S,E>>)`, `with_persister(Arc<dyn StateMachinePersister<S, E, String>>)`, `acquire_state_machine(&str)`, `release_state_machine(&str)`）、`persist::default_state_machine_persister::DefaultStateMachinePersister::new(factory: Arc<dyn StateMachineFactory<S, E>>, persist: Arc<dyn StateMachinePersist<S, E, T>>)`、`state_machine_persist::StateMachinePersist`（注意：在 crate 根模块，不在 `persist` 下）、`DefaultStateMachineContext::new(id: String, state: S) -> Self`、`TransitionData::with_all(1, 2, None, None, Some(20), Some(1), Vec::new(), None, TransitionKind::External, None, "timer")`。
- Produces: 二进制 `state_machine_service`；README 的"持久化与恢复"小节引用它。

- [ ] **Step 1: 注册二进制**

```toml
[[bin]]
name = "state_machine_service"
path = "state_machine_service.rs"
```

- [ ] **Step 2: 运行以确认失败**

Run: `cargo run -p next-web-examples --bin state_machine_service`
Expected: 失败，缺少文件。

- [ ] **Step 3: 写示例**

要求：定义示例内 `struct MemoryStore { slot: Mutex<Option<i32>> }`，按下列**已核对的签名**实现 `StateMachinePersist<i32, i32, ()>`：

```rust
fn write(&self, context: &dyn StateMachineContext<i32, i32>, _context_obj: ()) -> Result<(), BoxError>
fn read(&self, _context_obj: ()) -> Result<Arc<dyn StateMachineContext<i32, i32>>, BoxError>
```

`read` 无值时返回 `Box::new(std::io::Error::other("nothing was persisted"))`；有值时用 `DefaultStateMachineContext::new(id, state)` 构造（`id` 用 `context.id().to_owned()`，`state` 用 `*context.state()`）。机器用 20ms 定时转移（`1 --timer--> 2`）；流程：`acquire` → `tokio::time::sleep(50ms)` → `release`（触发持久化）→ 再次 `acquire` → 断言 `2` → 打印 `restored: 2`。

- [ ] **Step 4: 运行以确认通过**

Run: `cargo run -p next-web-examples --bin state_machine_service`
Expected: 输出 `restored: 2`，退出码 0。

- [ ] **Step 5: 提交**

```bash
git add examples/Cargo.toml examples/state_machine_service.rs
git commit -m "docs(examples): add the service and persistence example"
```

---

### Task 4: 迁移处理器示例 `examples/state_machine_handler.rs`

**Files:**
- Create: `examples/state_machine_handler.rs`
- Modify: `examples/Cargo.toml`

**Interfaces:**
- Consumes: `#[next_web::on_transition(source = 1, target = 2, event = 100)]`（零参数形式，默认目标 `::next_web::state_machine`）、生成的 `<name>_handler()`、`StateMachine::add_transition_handler(Arc<OnTransition<S, E>>)`、`next_web::state_machine::handler::on_transition::OnTransition`、`next_web::state_machine::state_context::StateContext`。
- Produces: 二进制 `state_machine_handler`。

- [ ] **Step 1: 注册二进制**

```toml
[[bin]]
name = "state_machine_handler"
path = "state_machine_handler.rs"
```

- [ ] **Step 2: 运行以确认失败**

Run: `cargo run -p next-web-examples --bin state_machine_handler`
Expected: 失败，缺少文件。

- [ ] **Step 3: 写示例**

要求：文件级 `static AUDITS: AtomicUsize = AtomicUsize::new(0);`；用 `#[next_web::on_transition(source = 1, target = 2, event = 100)]` 标注
`fn audit(_ctx: &dyn next_web::state_machine::state_context::StateContext<i32, i32>) -> Result<(), next_web_core::error::BoxError>`，函数体 `AUDITS.fetch_add(1, Ordering::SeqCst);` 后返回 `Ok(())`；注册处写 `machine.add_transition_handler(Arc::new(audit_handler())).expect("the handler is registered")`；发 `100` 后断言 `AUDITS.load(Ordering::SeqCst) == 1` 且机器状态为 `2`，打印 `audited: 1`。文件顶部注释说明：宏默认目标为 `next_web::state_machine`，直接依赖状态机 crate 的项目需写 `crate = next_web_state_machine`。

- [ ] **Step 4: 运行以确认通过**

Run: `cargo run -p next-web-examples --bin state_machine_handler`
Expected: 输出 `audited: true`，退出码 0。

- [ ] **Step 5: 提交**

```bash
git add examples/Cargo.toml examples/state_machine_handler.rs
git commit -m "docs(examples): add the transition handler example"
```

---

### Task 5: crate README

**Files:**
- Create: `next-web-state-machine/README.md`

**Interfaces:**
- Consumes: Task 1–4 的示例名（`state_machine_basic`、`state_machine_hierarchy`、`state_machine_service`、`state_machine_handler`）与它们打印的文本。
- Produces: README 中的快速上手代码块（与 `examples/state_machine_basic.rs` 主体一致）、特性表、"不支持/未实现"清单。

- [ ] **Step 1: 写 README**

必须包含且仅包含以下小节：`# next-web-state-machine`（一句话定位）、`## Quick start`（Task 1 的代码块 + `cargo run --example state_machine_basic` 输出）、`## What it supports`（状态/转移、守卫与动作、层级子机、正交区域、延迟事件、fork/join、历史状态、定时转移、扩展状态、监听器/拦截器/监视器、安全规则、持久化、service 层、`#[on_transition]`）、`## Configuration`（`ConfigurationData` 的 13 个参数各一句）、`## Not supported`（并行区域执行 = 构建期拒绝；分布式 `ensemble` = 拒绝；`ImmediateCancel` 无法被 exit 中断；状态动作内联执行不可并发取消）、`## Examples`（四个示例的命令与预期输出）、`## Feature flags`（`next-web` 侧 `enable-state-machine`）、`## License`（指向 workspace）。

- [ ] **Step 2: 验证 README 中的命令可执行**

Run: 依次执行 Quick start 与 Examples 小节列出的 4 条 `cargo run` 命令。
Expected: 每条退出码 0，输出与 README 写的一致。

- [ ] **Step 3: 验证没有死链**

Run: `cargo doc -p next-web-state-machine --no-deps`（带 `CARGO_TARGET_DIR`）
Expected: 0 告警（README 不进 doc，此步确认 crate 文档未被破坏）。

- [ ] **Step 4: 提交**

```bash
git add next-web-state-machine/README.md
git commit -m "docs(state-machine): add the crate readme"
```

---

### Task 6: CHANGELOG 与迁移说明

**Files:**
- Create: `next-web-state-machine/CHANGELOG.md`

**Interfaces:**
- Consumes: 本周期已合并的破坏性变更清单（逐条必须写进 `## Migration from 0.1`）：`Region::send_event` 返回值由 `Vec<Arc<dyn StateMachineEventResult>>` 变为 `Arc<dyn StateMachineEventResult>`；消息类型统一为 `Arc<dyn Message<E>>`；`Region::event_lock(&self) -> &tokio::sync::Mutex<()>` 为**新增必需方法**；`ExtendedState::get_variables(&self) -> HashMap<K, AnyValue>`、`get(&self, &K) -> Option<AnyValue>`、新增 `set(&self, K, AnyValue)`、`set_extended_state_change_listener(&self, Arc<dyn _>)`；`StateMachine::restore_context(&self, &dyn StateMachineContext<S,E>)` 与 `reset_to` 现在会关闭 initial transition；新增 `StateMachineErrorKind::MachineNotFound`；`StateMachinePersister` 基于 `StateMachinePersist`；新增 `service` 与 `handler` 模块；`StateMachineConfigurationBuilder::persister` 需自行提供 `StateMachineRuntimePersister` 实现（crate 未内置）。
- Produces: `## 0.2.0` 小节 + `## Migration from 0.1` 小节。

- [ ] **Step 1: 写 CHANGELOG**

结构：`# Changelog`、`## 0.2.0`（`### Added` / `### Changed` / `### Fixed` 三个列表，逐条一句英文，`### Fixed` 必须包含：事件处理串行化、configurer 按类型键值并在构建后释放、持久化恢复扩展状态变量、`reset_to` 不再重放初始转移、`post_transition` 错误不再被吞、`RegionExecutionPolicy::Parallel` 现在被拒绝、配置的 `StateDoActionPolicy` 现在真正下发到状态）、`## Migration from 0.1`（按上面的清单逐条给出"旧 → 新"的签名对照）。

- [ ] **Step 2: 核对每条 Fixed 都有对应代码位置**

Run: 逐条用 `git log --oneline -30` 与源码位置核对，确认没有把未合并的改动写进 CHANGELOG。
Expected: 每条都能指出提交或文件。

- [ ] **Step 3: 提交**

```bash
git add next-web-state-machine/CHANGELOG.md
git commit -m "docs(state-machine): add the changelog and migration notes"
```

---

### Task 7: 版本提升到 0.2.0

**Files:**
- Modify: `next-web-state-machine/Cargo.toml:3`（`version = "0.1.0"` → `"0.2.0"`）
- Modify: `Cargo.toml`（`[workspace.dependencies]` 中 `next-web-state-machine = { path = "./next-web-state-machine" }` 追加 `, version = "0.2.0"`）

**Interfaces:**
- Consumes: Task 6 的 CHANGELOG 版本号（必须一致）。
- Produces: 可发布的 crate 元数据；umbrella 的 `version = "*"` 使用方无需改动。

- [ ] **Step 1: 修改两处版本**

只改 `next-web-state-machine`；`next-web`（0.2.2）与其它 crate 不动。

- [ ] **Step 2: 验证整仓仍可构建**

Run: `cargo check -p next-web-state-machine -p next-web --features enable-state-machine`
Expected: 退出码 0，无告警。

- [ ] **Step 3: 验证 workspace 依赖带版本后仍解析**

Run: `cargo metadata --no-deps --format-version 1 | Select-String "next-web-state-machine"`
Expected: 输出的 `version` 为 `0.2.0`。

- [ ] **Step 4: 提交**

```bash
git add next-web-state-machine/Cargo.toml Cargo.toml
git commit -m "chore(state-machine): release 0.2.0"
```

---

### Task 8: 死文件清理与全量验证

**Files:**
- Delete: `next-web-state-machine/src/processor/mod.rs`、`next-web-state-machine/src/processor/state_machine_handler_call_helper.rs`（代码引用已移除，仓库里仍被跟踪）、`next-web-state-machine/tests/allocation.rs`、`next-web-state-machine/src/.dsh_probe`

**Interfaces:**
- Consumes: 无。此任务只删除已无引用的文件。
- Produces: 干净的发布树；`git ls-files` 中不再包含上述路径。

- [ ] **Step 1: 确认无引用**

Run: `Select-String -Path next-web-state-machine/src/**/*.rs -Pattern 'processor::|allocation'`（在 PowerShell 中）
Expected: 无匹配（`src/lib.rs` 已无 `pub mod processor;`）。

- [ ] **Step 2: 删除（需要可写 shell，沙箱会在 `src/` 下拒绝删除）**

```bash
git rm -r next-web-state-machine/src/processor
git rm next-web-state-machine/tests/allocation.rs
rm -f next-web-state-machine/src/.dsh_probe
```

- [ ] **Step 3: 验证构建与测试仍全绿**

Run: `cargo test -p next-web-state-machine`
Expected: `37 lib + 4 concurrency + 4 configuration + 4 handlers + 3 fluent + 4 hot_path(+1 ignored) + 15 runtime + 7 service + 1 doctest` 全部 `ok`。

- [ ] **Step 4: 全量验证（最终门槛）**

依次执行，全部要求退出码 0：
`cargo clippy -p next-web-state-machine --all-targets`（0 告警）、
`cargo doc -p next-web-state-machine --no-deps`（0 告警）、
`cargo test -p next-web --features enable-state-machine --test state_machine_auto_configuration`（2 passed）、
`cargo check -p next-web`（**默认特性**，确认关闭 `enable-state-machine` 时 umbrella 仍可编译）、
`cargo run -p next-web-examples --bin state_machine_basic` / `state_machine_hierarchy` / `state_machine_service` / `state_machine_handler`（预期输出见 README）。

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "chore(state-machine): drop the dead processor and probe files"
```

---

## Out of Scope（本计划不做）

- **正交区域示例**：模型层如何声明多个区域（`StateData::submachine_factory` 的分组语义）尚未有集成测试锁定，先写"区域从模型构建"的测试再写示例，留作后续任务。
- **测试盲区**（并发 fork/join、并发延迟事件、三级嵌套、跨区域作用域转移、守卫拒绝后转发的双执行风险）：独立计划。
- **默认 `StateMachineRuntimePersister` 实现**、`send_events`（批量发送）的实现或删除：独立任务。
- **`next-web-macros` 47 条 clippy 存量、`next-web` 1 条旧告警**：属于 workspace 级清零，不在本计划。
- **推送与 CI**：需要仓库写权限，由人工执行。
