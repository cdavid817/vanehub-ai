# P0：让用户配置的权限与路径限制真正生效

变更 ID：`enforce-loop-execution-scope`  
目标仓库：<https://github.com/cdavid817/vanehub-ai>  
分析基线：`75f23d81406bdc1fd4d6b4d5a41fbf90e395c4fc`  
交付性质：待实施的 OpenSpec 变更包；不包含产品实现，不表示现有漏洞已经修复。

## 使用

将 ZIP 解压至仓库根目录。所有内容都在一个新的 `openspec/changes/enforce-loop-execution-scope/` 目录内，不覆盖根 README、主规范或其他变更。若同名变更已经存在，先比较再合并，不要覆盖已有工作。

在仓库根目录运行：

```bash
npx --yes @fission-ai/openspec@1.8.0 validate enforce-loop-execution-scope --strict
npx --yes @fission-ai/openspec@1.8.0 show enforce-loop-execution-scope
```

阅读 `proposal.md`、`design.md` 和 `acceptance.md`，按 `tasks.md` 实施。需要交给编码 Agent 时，把 `implementation-prompt.md` 的正文作为任务输入。实施后再运行仓库规定的完整检查；本包只完成规范校验，验证记录见 `verification.md`。

## 文件

| 文件 | 用途 |
|---|---|
| proposal.md | 问题、范围、兼容性和六项规范能力 |
| design.md | 权限顺序、路径语义、CLI 能力边界、数据与执行流程 |
| tasks.md | 按依赖排序的可勾选实施任务 |
| acceptance.md | 正反例、竞态、迁移、平台及验收证据 |
| specs/*/spec.md | 一个新增能力和五个现有能力的规范增量 |
| implementation-prompt.md | 可直接使用的实施任务提示词 |
| verification.md | 本次规范校验结果与未执行项 |

## 三个不可妥协的结果

1. 所有 VaneHub 托管的变更操作，必须在真实副作用发生前检查本次运行的路径上限；`trusted`、`yolo` 和人工批准均不能突破它。
2. ACP 文件读取遇到 `Ask`、策略异常、过期批准时，不得提前读取或返回文件内容。
3. 未证明覆盖全部变更通道的 CLI、验证命令和 MCP，不能显示为“强制限制已生效”；路径违规或证据不完整的运行，不能被人工点击接受为成功。

默认使用 `preventive-required`，即要求执行前约束。兼容模式 `artifact-audited` 只允许用户在每次手动启动时明确接受“未托管通道只有结果检查”的限制；它不会获得强制隔离标记，也不会放宽 Verifier 只读要求。完整模式边界见设计。
