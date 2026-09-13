# P0：统一记忆注入与 recall 的读取范围

Change ID：`unify-memory-read-scope`  
仓库：[cdavid817/vanehub-ai](https://github.com/cdavid817/vanehub-ai)  
核对基线：`main@75f23d81406bdc1fd4d6b4d5a41fbf90e395c4fc`，2026-09-09；只读远端 main 与本地 SHA 一致。  
交付：待实施 OpenSpec 与 Claude Code prompt，不含产品实现。

核心决策：保留一个共享记忆存储池，复用现有 memory read、global-memory access、会话模式、scope 和 audience；由可信运行上下文决定可读全集，注入、正文选择、recall、Context Engine 分别在该全集内排序与限额。不新增另一套 readScope 设置，不把前 200 条注入引用当作完整授权列表。

## 使用

ZIP 解压到仓库根目录后，内容只新增 `openspec/changes/unify-memory-read-scope/`。同名目录已存在时先比较，不能覆盖已有工作。阅读 proposal/design/acceptance，按 tasks 实施，完整一段提示词在 `implementation-prompt.md`。

```bash
npx --yes @fission-ai/openspec@1.8.0 validate unify-memory-read-scope --strict
npx --yes @fission-ai/openspec@1.8.0 show unify-memory-read-scope
```

## 已确定的行为

| 会话/策略 | 全局 active | 当前工作区 active | 其他工作区 |
|---|---|---|---|
| standard，允许读全局 | audience 匹配才允许 | audience 匹配才允许 | 拒绝 |
| standard，关闭全局读取 | 拒绝 | audience 匹配才允许 | 拒绝 |
| project-only，有可信工作区 | 拒绝 | audience 匹配才允许 | 拒绝 |
| temporary / memory read 关闭 | 拒绝 | 拒绝 | 拒绝 |
| 身份、工作区或治理健康状态无法确认 | 拒绝 | 拒绝 | 拒绝 |

明确没有工作区的 standard 会话仍可按策略读取 global；“没有工作区”与“工作区解析失败”不同。candidate、archived、deleted、malformed、quarantined 永不交付给 Agent。作者/来源字段只做溯源。

`strengthen-governed-cross-session-memory` 已提出 governed recall，本包抽出并补全读取子范围；冲突衔接见 design.md 第 9 节。此次不处理抽取任务、候选审批事务或多 Agent memory episode。
