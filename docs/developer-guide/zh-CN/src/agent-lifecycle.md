# Agent 生命周期与 provider 运行时

本章覆盖已注册 Agent(内置的 OnePiece 除外)如何被编辑,以及运行时如何在不引入应用层 provider 身份分支的前提下,将一个稳定的 Agent id 解析为具体的 provider 契约。

## 编辑已注册的 API Agent

用户创建的 API Agent 的显示名称、模型 id、Base URL 与存储的 API key 是可编辑的。该 Agent 的 `id`、`provider` 与 `interface format` 通过普通编辑操作是不可变的。编辑会像注册一样重新校验:针对 `openai-compatible` Agent 省略必需的 Base URL 会拒绝整个编辑,不会持久化其中任何一部分。轮换的 API key 会替换已存储的凭据,并在下一次生成时生效。

OnePiece 是例外:它使用由目录支持的专用 provider **Profile** 操作,保留稳定的 id `onepiece`,同时允许配置多个各自独立受保护的 provider/endpoint/model 组合,以及一个显式的活跃 Profile。OnePiece 的 provider、endpoint 类型、interface format 与 Base URL 都从所选的内置目录条目解析——从不被直接编辑。

## 稳定的 provider 解析

Agent 运行时通过一个 **provider registry** 来解析受支持的内置 CLI 运行时行为,该 registry 以 Agent registry 条目的稳定 id 为键。与 provider 无关的应用与 Session 模块不会根据 provider 身份分支来选择行为。一个没有兼容 provider 注册的 Agent id 会返回一个分类好的 `unsupported-provider` 错误,且不会回退到其他 provider。

## Provider 元数据与能力

每个注册的 provider 各自声明经过校验的元数据、就绪前提与受支持的运行时能力(interaction、resume、structured-output、terminal、usage、permission、model-selection、reasoning),独立于显示名称匹配或调用方推断。provider 未声明的能力不会被静默假设为存在。

## 设计所在

本章用于为贡献者定向。权威的需求位于规范中。

- [openspec/specs/agent-lifecycle-management](../../../../openspec/specs/agent-lifecycle-management/spec.md)
- [openspec/specs/agent-provider-runtime](../../../../openspec/specs/agent-provider-runtime/spec.md)
- [openspec/specs/agent-switching](../../../../openspec/specs/agent-switching/spec.md)

native 执行路径位于 `agent_runtime` bounded context 中;见 [Native bounded context](native-contexts.md)。
