# 工具注册表与执行

Native API Agent(包括 OnePiece)接收一个固定的、与 provider 无关的工具目录,运行时驱动一个多轮 tool-use 循环,直到 provider 返回一个终态响应。来自 MCP 的工具叠加在这个固定目录之上——参见 [MCP 工具与客户端](mcp-tools.md)。

## 固定的 native 工具目录

每次 native API 生成(`launch_kind = api`)都在其发出的 provider 请求中声明同一个固定工具集:

- `shell` —— 命令执行
- `file` —— 读/写
- 内容搜索
- 文件名搜索
- 限定范围的文件编辑
- 跨会话内存

每个工具只定义一次,并按会话 `interface_format` 所要求的请求形态转换:

- `anthropic` → `{name, description, input_schema}`
- `openai-compatible` → `{type: "function", function: {name, description, parameters}}`

## 多轮 tool-use 循环

当 provider 响应请求一个或多个工具调用时,运行时执行这些调用并将其结果作为新的一轮回传,如此重复,直到 provider 返回一个不再包含工具调用的响应。没有工具调用的响应即为该次用户消息的终态响应,等同于一次不带工具的生成。该循环受每条用户消息固定最大往返次数约束;超出该上限会被显式处理,而不是无限循环下去。

## Skill 提供的工具

Skill 可以在固定目录之上贡献自己的工具,这些工具在沙箱中而非宿主进程中执行。相关需求见 [openspec/specs/skill-tool-runtime](../../../../openspec/specs/skill-tool-runtime/spec.md)。

## 设计所在之处

本章用于为贡献者定位。权威需求位于规范之中。

- [openspec/specs/agent-tool-execution](../../../../openspec/specs/agent-tool-execution/spec.md) —— 固定目录、按格式转换与 tool-use 循环。
- [openspec/specs/agent-tool-registry](../../../../openspec/specs/agent-tool-registry/spec.md) —— 已注册的 Agent 目录与能力元数据。
- [openspec/specs/skill-tool-runtime](../../../../openspec/specs/skill-tool-runtime/spec.md) —— Skill 所提供工具的沙箱化执行。

工具执行位于 `agent_runtime` 限界上下文中;参见 [Native 限界上下文](native-contexts.md)。

### 历史决策记录

这些记录的是在某个时间点所作的决策,并不作为当前叙述来维护。在此链接是为了让它们可被触达而非沦为孤儿;上面的规范仍然是权威的。

- [Skill Tool Runtime Security](../../../architecture/skill-tool-runtime-security.md) —— 沙箱化 Skill Tool 运行时发布时所记录的依赖审查、验证证据、上线与回滚。
