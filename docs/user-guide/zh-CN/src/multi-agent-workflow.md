# 多 Agent 编码工作流预览

**状态：预览——runtime/service contract 已存在，创建 UI 尚不可用。**

预期工作流把一个编码目标拆成依赖图：

```text
plan
  +--> frontend（primary: codex-cli，fallback: claude-code）--+
  +--> native  （primary: claude-code，fallback: codex-cli）--+--> test --> review
```

`frontend` 与 `native` 节点可以独立就绪；`test` 等待两者输出，`review` 再等待测试完成。

每个节点记录：

- 稳定节点 id；
- primary 稳定 Agent id；
- 有序 fallback Agent id；
- 指令；
- 前置节点 id。

成功的前置输出会携带来源节点和实际 Agent 信息传给下游。可重试执行失败会依序进入 fallback；校验、策略、取消、持久化和上下文上限失败不会启动 fallback。

取消操作会停止当前 attempt 并阻止新 attempt。桌面端状态持久化到 SQLite，Web/mock 状态仅为模拟。

这里有意不提供点击式创建步骤或 UI 截图，因为正常创建会话 UI 仍将多 Agent 标记为不可用。只有在用户可见控件与 Playwright 路径能够创建、观察并完成或取消一次运行后，本章才可升级为已交付工作流。
