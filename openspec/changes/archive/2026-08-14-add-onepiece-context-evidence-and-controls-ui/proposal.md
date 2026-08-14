## Why

OnePiece 已能自动判断、优化和压缩上下文，但用户仍看不到一次压缩究竟节省了多少上下文，也缺少一个可持久化的全局开关来避免在不合适的工作流中自动压缩。第四阶段需要把内容无关的压缩证据投影到会话 UI，并让桌面与 Web/mock 运行时共享同一套用户控制语义。

## What Changes

- 为成功的自动上下文压缩生成结构化、内容无关的证据投影，包含压缩前后字符数、可用时的 token 数、测量质量、节省量、触发来源、压缩路径和策略版本。
- 在聊天记录中以现有 rich card 展示压缩证据，并随消息历史保留；卡片不得包含原始提示词、工具载荷、模型输出或密钥。
- 在 OnePiece 参数页增加“自动上下文压缩”持久化开关，默认启用，并显示其只影响后续生成的作用域。
- 通过既有 settings service 边界同步桌面 SQLite 与 Web/mock 存储；React 组件不直接调用 Tauri API。
- 将用户设置与请求级抑制、冷却和熔断共同纳入压缩决策；关闭后不得优化、摘要或修改后续 OnePiece 请求上下文。
- 为桌面和 Web/mock adapter 提供契约一致的抑制行为与安全证据卡片。

## Capabilities

### New Capabilities

- `agent-context-evidence-projection`: 定义成功压缩的安全证据 DTO、指标语义、聊天投影和敏感内容边界。

### Modified Capabilities

- `agent-context-compaction-control`: 将持久化用户开关纳入自动压缩抑制优先级与运行作用域。
- `app-settings`: 扩展共享设置模型，持久化自动上下文压缩开关并保持桌面/Web adapter 对称。
- `settings-cli-management-ui`: 在 OnePiece 参数页提供可访问的自动压缩控制与作用域说明。
- `chat-experience`: 在会话历史中展示并保留成功压缩的证据卡片。

## Impact

- 前端：设置类型与校验、settings provider、Tauri/Web settings adapters、OnePiece 参数页、国际化资源、rich card 测试和 Web/mock 生成模拟。
- Native：桌面设置 DTO/映射/持久化、个性化设置快照、OnePiece 自动压缩编排与证据事件。
- Runtime：桌面与 Web/mock 都受影响；设置通过现有服务边界传递，不新增 React 到 Tauri 的直接依赖。
- API/数据：新增一个默认启用的布尔设置字段；旧安装缺失该字段时安全回退为启用。不引入新依赖或独立数据表。
