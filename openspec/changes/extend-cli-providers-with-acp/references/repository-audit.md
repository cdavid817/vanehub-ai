# 仓库审阅基线与实现导航

基线：`main@34a449e80a1badc16fcfb49dcbad26134aecf2d9`，读取日期 2026-09-06。通过 GitHub 读取相关源码与规范；这是定向审阅，不是全仓库运行结果。当前任务仅生成实施包，未修改或提交仓库。

| 已读取路径 | 审阅结论 |
| --- | --- |
| [AGENTS.md](https://github.com/cdavid817/vanehub-ai/blob/34a449e80a1badc16fcfb49dcbad26134aecf2d9/AGENTS.md) | 统一入口、React 19、npm、分层、不可改归档、完整 CI 命令；适用当前工作树更近规则。 |
| [openspec/config.yaml](https://github.com/cdavid817/vanehub-ai/blob/34a449e80a1badc16fcfb49dcbad26134aecf2d9/openspec/config.yaml) | spec-driven；requirement SHALL/MUST；四级 Scenario；任务 X.Y。配置中的旧验证示例不替代更近 AGENTS.md。 |
| [openspec/specs/provider-plugin-sdk/spec.md](https://github.com/cdavid817/vanehub-ai/blob/34a449e80a1badc16fcfb49dcbad26134aecf2d9/openspec/specs/provider-plugin-sdk/spec.md) | 静态 SDK、V1 data-only manifest、外部 Provider 禁止、合约/解析/兼容测试；本包精确保留四个被修改 requirement 的现有 scenario 标题。 |
| [openspec/specs/cli-environment-management/spec.md](https://github.com/cdavid817/vanehub-ai/blob/34a449e80a1badc16fcfb49dcbad26134aecf2d9/openspec/specs/cli-environment-management/spec.md) | source-aware 目录、探测、PATH-selected/recommended、正交状态和 action plan；本包追加需求，不替换既有来源逻辑。 |
| [src-tauri/src/contexts/tooling/cli/domain/registry.rs](https://github.com/cdavid817/vanehub-ai/blob/34a449e80a1badc16fcfb49dcbad26134aecf2d9/src-tauri/src/contexts/tooling/cli/domain/registry.rs) | CLI_TOOL_DEFINITIONS 为五个原有 CLI，受审计源/可执行名/探针定义在此。 |
| [src-tauri/src/contexts/agent_runtime/infrastructure/providers/compatibility.rs](https://github.com/cdavid817/vanehub-ai/blob/34a449e80a1badc16fcfb49dcbad26134aecf2d9/src-tauri/src/contexts/agent_runtime/infrastructure/providers/compatibility.rs) | 固定五个 compatibility definitions、统一 manifest 构造与 provider 接口实现。 |
| [src-tauri/src/contexts/agent_runtime/infrastructure/providers/manifest.rs](https://github.com/cdavid817/vanehub-ai/blob/34a449e80a1badc16fcfb49dcbad26134aecf2d9/src-tauri/src/contexts/agent_runtime/infrastructure/providers/manifest.rs) | V1 deny_unknown_fields，拒绝执行类字段，当前 usage 声明要求及能力归一化。 |
| [src-tauri/src/contexts/agent_runtime/infrastructure/providers/invocation.rs](https://github.com/cdavid817/vanehub-ai/blob/34a449e80a1badc16fcfb49dcbad26134aecf2d9/src-tauri/src/contexts/agent_runtime/infrastructure/providers/invocation.rs) | 受管与交互启动 grammar、UnsupportedAgent、权限治理 ID 列表；必须追踪新入口是否漏映射。 |
| [src-tauri/src/contexts/agent_runtime/infrastructure/providers/mod.rs](https://github.com/cdavid817/vanehub-ai/blob/34a449e80a1badc16fcfb49dcbad26134aecf2d9/src-tauri/src/contexts/agent_runtime/infrastructure/providers/mod.rs) | 既有 output parser、invocation 和 session_capture 导出位置；不得另开平行日志或捕获系统。 |
| [docs/developer-guide/zh-CN/src/runtime-boundaries.md](https://github.com/cdavid817/vanehub-ai/blob/34a449e80a1badc16fcfb49dcbad26134aecf2d9/docs/developer-guide/zh-CN/src/runtime-boundaries.md) | 现有 service/native/mock 边界和尚未实现 ACP 的文档叙述；实现后更新，保留 MCP/LSP/ACP 区分。 |
| [package.json](https://github.com/cdavid817/vanehub-ai/blob/34a449e80a1badc16fcfb49dcbad26134aecf2d9/package.json) | 实际 lint:ci/contracts:check/architecture:check/docs:check/desktop 等脚本。 |

## 执行前必须继续追踪

下列为搜索目标，不保证文件/符号名在新分支保持不变：AgentProvider/ProviderRegistry/ProviderCapabilities；Agent Runtime application ports；bootstrap/cli；CLI 参数声明和生成器；session capture 与 usage；权限最终投影；创建会话和 CLI 管理组件；multi-agent/group-chat、scheduled task 与 cli_delegation；MCP/Skill/role briefing 注入；database migrations；platform process；统一 logging/operations。

React service 聚合入口与拆分后的子 service 应以当前代码为准。不要只修改 tauri-agent-client.ts 或一个枚举而漏掉实际使用的拆分 adapter。用调用关系与测试确认覆盖。

## 分支与差异处理

不假设存在 dev 分支，不自动 checkout main，也不 reset 到此基线。当前工作区若已包含部分新 CLI/ACP 实现，先记录差异并复用。保留用户未提交工作；有冲突时停下冲突部分并继续不受影响的验证，不删除用户代码。

## 拟新增位置（非已有实现）

协议 infrastructure 可放在现有 providers 下的 acp/ 子模块；Provider 专属 glue 使用小文件；application 新 port 优先扩展现有会话契约；UI 通过现有 service/DTO。具体文件名由当前 DDD module 规则确定，不依赖本包不存在的假 API。
