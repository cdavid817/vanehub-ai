# Skill Tool 运行时安全

> **本章是某一时点的记录**,不作为持续维护的叙述。它记录的是沙箱化 Skill Tool 运行时发布时的依赖审查、验证证据、上线与回滚,审查时间为 2026-08-17。持续维护的需求见 [openspec/specs/skill-tool-runtime](../../../../openspec/specs/skill-tool-runtime/spec.md);两者不一致时以规范为准。

## 功能边界

`skill-tool-module-runtime` 默认关闭。不带该 feature 的构建仍保留声明式 Skill 工具的发现、校验、完整性、信任与治理能力。WebAssembly 条目被报告为 `module-runtime-unavailable`,绝不隐藏,也绝不当作可成功执行。

启用该 feature 会引入 `wasmtime 47.0.3`,默认 features 全关,只开 `cranelift`、`runtime` 与 `std`。运行时不链接 Wasmtime 的 WASI、component-model、cache、profiling、pooling、thread 或网络集成。宿主能力必须经由 VaneHub 自有的带类型网关提供。

## 依赖审查

选择该引擎的理由是:它为 fuel、epoch 中断、store 限额与内存控制提供了持续维护的 Rust 嵌入 API,有明确的安全策略,并且被持续 fuzz。47.0.3 版要求 Rust 1.94,与仓库的 Rust 1.97.1 工具链兼容。之所以精确锁版,是因为沙箱行为与安全修复必须在升级前经过审查。其声明的许可证是 `Apache-2.0 WITH LLVM-exception`。

`wasmi 2.0.0-beta.10` 被否决,因为可用发行版仍是 beta,而且选用解释器并不能免除对宿主能力与资源的强制约束。`jsonschema 0.49.9` 经过评估但未引入:仓库已经实现了刻意收窄的 Skill schema 子集,而该 crate 的默认 features 包含 HTTP/文件解析与 TLS。引入一个通用解析器会扩大依赖面与网络面,却并不实现任何必需的 schema 能力。

lockfile 必须通过 `cargo audit`,所选 feature 图中不得存在有漏洞的包。许可证审查必须覆盖启用 feature 后的依赖图,任何 Wasmtime 升级都必须重做一次公告、许可证、feature、MSRV、恶意模块与二进制体积审查。即使未来某个依赖使 Wasmtime WASI 可传递获得,它仍然被禁止。

2026-08-17 的那次审查只解析出 `Apache-2.0 WITH LLVM-exception` 下的 Wasmtime 47.0.3 相关 crate;`cargo tree` 既未找到 `wasmtime-wasi` 也未找到 `wasi-common`。`cargo audit 0.22.2` 报告零漏洞,并在既有桌面依赖图中给出 18 条被允许的告警。这些告警涉及遗留的 GTK3 绑定、`proc-macro-error`、若干 `unic-*` crate、`event-listener` 的 RUSTSEC-2026-0221 与 `glib` 的 RUSTSEC-2024-0429;没有一条是由 Wasmtime 依赖图引入或位于其上的。它们仍属仓库依赖治理的后续工作,本次变更没有把它们抑制掉。

## 验证证据

- Linux `x86_64-unknown-linux-gnu`,Rust 1.97.1:仅声明式的定向套件通过 138 项测试;启用模块的定向套件通过 143 项测试。
- 确定性的结构化测量通过了 manifest 聚合预算、受控的文件系统与字节耗尽、取消计账、原子的宿主预算并发,以及每个 Skill 的模块并发测试。这些检查断言的是精确的准入计数与硬上限,而不是耗时毫秒数。
- Playwright 通过 137 项测试,含 futuristic/minimal 与 desktop/narrow 视觉变体。Linux 原生 Desktop Smoke 穿过了真实的 Tauri IPC 边界,1/1 场景通过;桌面 harness 单元测试 11/11 通过。
- 原生平台状态:Linux `PASSED`;Windows `PASSED`;macOS `PASSED`。Linux 验证在本地跑过,又在 CI 跑了一次。GitHub Actions 的 `32000007318` 次运行在三个平台上都通过了原生 Desktop Smoke。其 `windows-latest` 的 Native Check 还通过了原生构建,以及仅声明式与模块运行时两套恶意 Skill Tool 套件。这些状态只针对真正跑过它们的平台报告。

## 上线

保持模块 feature 默认关闭。先上线声明式工具,再只在经过审查的原生构建里启用模块 feature。运维人员对每一个不可变 revision 分别做校验、授信与启用。全局与单 Skill 两级 kill switch 可原子地撤下可执行条目,同时保留信任、校验、诊断与审计证据。

## 回滚

关闭 `skill-tool-module-runtime` 会在编译期移除模块执行,同时保留 manifest、校验状态、信任记录、诊断与仅声明式的运行能力。不需要迁移任何已存储的 Skill 数据。

发生运维事故时,先停用受影响的那个 Skill;范围不确定时再用全局开关。当某个 revision 的完整性或来源可疑时,吊销该精确 revision。恢复需要一次干净的重新发现与校验、必要时一个显式的新授信决定,以及一次单独的启用动作。
