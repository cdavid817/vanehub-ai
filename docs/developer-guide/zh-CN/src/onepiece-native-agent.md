# OnePiece native Agent

OnePiece 是 VaneHub 内置的第一方 Agent。与基于 CLI 的 Agent 不同,它完全通过 native API 运行时运行:`launch_kind = api`、`agent_origin = builtin`,预留稳定 id 为 `onepiece`。它在首次启动时被植入注册表,即便尚未存在任何 provider 配置或凭证时也保持可见。

## 身份与生命周期

OnePiece 身份由注册表拥有,而非由 provider 配置拥有。它与多个命名、由 catalog 支撑的上游 provider **Profile** 相分离,每个 Profile 独立保管自己的凭证。同一时刻至多有一个 Profile 被显式激活用于运行时生成。创建 Profile 时必须选择一个由所选 provider 拥有且经过评审的 endpoint 类型——不接受用户随意提供 provider 身份、接口格式或 Base URL。

## 设计所在

本章用于为贡献者定向。权威需求——稳定身份、注册表植入、预留 id 冲突处理、Profile 生命周期以及 provider-directory 契约——位于 spec 中。

- [openspec/specs/onepiece-native-agent](../../../../openspec/specs/onepiece-native-agent/spec.md)

与 CLI Agent 配置共享的 provider 目录以及 native API 运行时,在 [Runtime and service boundaries](runtime-boundaries.md) 与 [Native bounded contexts](native-contexts.md) 中介绍。
