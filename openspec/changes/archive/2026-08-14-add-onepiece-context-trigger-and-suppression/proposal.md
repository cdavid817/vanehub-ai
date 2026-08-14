## Why

OnePiece 已经能够测量、分类并优化上下文，但自动压缩仍由固定字符阈值单独决定，无法利用已验证的模型容量，也没有明确的抑制、冷却或失败保护边界。现在需要把 shadow 决策提升为可控的生产决策，同时保证未知容量、连续失败和调用方要求不压缩时仍能安全退化。

## What Changes

- 将已知模型容量与可信上下文测量用于自动压缩决策，并为未知或不足证据保留确定性的字符阈值回退。
- 引入请求级自动压缩控制，使调用方可以显式抑制自动压缩，而不影响手动压缩语义。
- 增加 generation 内自动压缩 cooldown，避免上下文未显著增长时重复压缩。
- 增加连续失败熔断；达到边界后，本 generation 内停止继续自动压缩并保持原始上下文。
- 通过统一日志记录内容无关的触发、回退、抑制、cooldown 与熔断证据和稳定 reason code。
- 本阶段仅影响 Tauri/native OnePiece provider runtime；Web/mock 继续保持现有确定性模拟行为。证据 UI 与 provider-native cache edit 留待后续变更。

## Capabilities

### New Capabilities

- `agent-context-compaction-control`: 定义请求级自动压缩抑制、generation cooldown、失败熔断及其安全诊断。

### Modified Capabilities

- `agent-context-measurement`: 将版本化 Token-aware 决策从只读 shadow 证据提升为证据充分时的生产触发输入。
- `agent-context-compaction`: 用 Token-aware 决策控制已知容量模型的自动压缩，并在证据不足时回退到现有字符阈值。

## Impact

- 主要影响 `src-tauri/src/contexts/agent_runtime/` 的上下文测量、API process adapter、压缩状态与测试。
- 自动压缩控制保持在 native runtime/application boundary 内；React 组件不会直接依赖 Tauri API，也不新增前端直连能力。
- 不引入新依赖、不修改 SQLite schema、不改变 provider request protocol；Web adapter 本阶段无需行为变更。
