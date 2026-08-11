# 案例教程：验证同一会话中的多 Agent 协作

本教程用一个「架构师 → 实现者 → 代码审查」案例，验证多个 Agent 是否真的出现在**同一个会话**里，以及成员身份、发言状态、`@` 交接和历史归属是否正确。

完成后，你应该能回答四个问题：

1. 创建会话时能否为不同角色选择 Agent？
2. 进入会话后能否同时看到所有成员和当前发言者？
3. 成员加入或离席后，已有消息的发言者身份是否保留？
4. 单 Agent 会话是否仍保持原来的界面和行为？

## 准备测试环境

在 PowerShell 中进入功能 worktree：

```powershell
Set-Location 'D:\cdavid\Documents\code\vanehub-ai-multi-agent'
```

先运行最快的一组自动化检查：

```powershell
npm run lint:ci
npm run test
npm run build
npx playwright test tests/e2e/multi-agent-session.spec.ts
```

预期结果：lint、单元测试和构建成功；多 Agent 专项 Playwright 用例全部通过。

> Web/mock 适合验证界面、成员增删和 `@` 补全，但不会启动真实 CLI。真实的 Agent 回复与自动交接必须使用 Tauri 桌面端。

## 案例目标

本案例只要求 Agent 评审一个小改动方案，不要求它们实际修改仓库：

> 为当前项目设计一个 `/health` 健康检查接口。架构师定义接口和边界，实施者给出实现步骤，代码审查检查测试、安全性和兼容性。

这样既能产生连续的角色交接，又不会为了测试 UI 引入无关代码。

## 第一步：启动应用

### 只验证 UI

```powershell
npm run dev
```

打开终端显示的地址，通常是 `http://127.0.0.1:5173/`。

### 验证真实 Agent 交接

```powershell
npm run tauri:dev
```

开始前确认计划使用的 CLI 已安装并完成认证。只有一个 CLI 可用时，也可以把三个角色都绑定到同一个 Agent；席位身份仍然由角色区分。

## 第二步：创建三个角色的会话

1. 点击**新建**。
2. 选择项目目录 `D:\cdavid\Documents\code\vanehub-ai-multi-agent`。
3. 会话标题填写 `多 Agent 健康检查评审`。
4. 会话类型选择**多 Agent**。
5. 配置以下席位：

| 顺序 | 角色 | Agent 建议 | 责任 |
| --- | --- | --- | --- |
| 1 | 架构师 | Claude Code、Codex CLI 或任一可用 Agent | 定义接口、边界和约束 |
| 2 | 实现者 | Codex CLI 或任一可用 Agent | 给出落地步骤与测试方案 |
| 3 | 代码审查 | 尽量选择不同模型家族 | 检查遗漏、风险和回归 |

6. 点击**创建**。

### 检查点 A：创建界面

- 至少保留两个席位时才能形成多 Agent 会话。
- 每个席位都能选择角色和 Agent。
- 添加、删除席位不会导致其他席位的选择丢失。
- 删除到只剩一个席位时，最后一个删除操作应被禁用。

## 第三步：检查同一会话中的成员展示

进入会话后，观察聊天区顶部和右侧信息面板。

### 检查点 B：协作房间头部

- 左侧会话列表中，该会话标题旁显示「多 Agent」标签；单 Agent 会话没有这个标签。
- 标题下方显示**多 Agent**和成员数量。
- 成员条同时列出三个角色，而不是只显示第一个 Agent。
- 每个成员同时显示角色名和 Agent 身份。
- 当前发言者具有高亮边框、状态点和「处理中」文字；状态不应只靠颜色表达。
- 生成状态应显示在会话头部，停止生成后恢复为「就绪」。

### 检查点 C：右侧成员管理

- **会话成员**标题旁显示当前人数。
- 每行显示角色、Agent 和模型家族。
- 添加成员时，Agent 与角色选择器都有可访问名称。
- 离席按钮有明确的悬停提示，且不会允许会话变成零成员。

## 第四步：执行角色交接

在输入框发送下面的提示：

```text
我们只评审方案，不修改文件。
请先以架构师身份定义 /health 接口的响应、失败边界和兼容性约束。
回复最后一行请单独写 @实现者，把下一步交给实现者。
```

实现者回复后，让它继续交接：

```text
请给出最小实现步骤和必须增加的测试。
回复最后一行请单独写 @代码审查。
```

最后要求代码审查收尾：

```text
请审查前面的方案，列出阻塞问题和建议，然后用 @用户 done 结束本轮。
```

### 检查点 D：交接和消息身份

- 输入 `@` 时出现会话成员补全，而不是文件补全。
- 轮次状态栏依次显示架构师、实现者和代码审查正在处理。
- Agent 消息左侧使用对应 Agent 的品牌图标。
- 消息头显示「角色名 · Agent 名称」，用户消息不应冒充任何席位。
- 三个角色的消息都出现在同一条时间线里，不应打开三个独立聊天页。

> 模型不一定每次都严格输出指定的 `@`。如果没有触发交接，重新明确要求「最后一行只写目标句柄」，并确认使用的是桌面端而不是 Web/mock。

## 第五步：验证成员加入、离席和历史归属

1. 在右侧**会话成员**区域再添加一个成员。
2. 确认聊天头部的成员数量和成员条立即更新。
3. 让新成员至少产生一条回复。
4. 点击该成员的离席按钮。
5. 展开**已离席成员**。

### 检查点 E：稳定身份

- 离席成员从当前成员条消失，并出现在已离席列表。
- 离席前的消息仍显示原来的角色名和 Agent 名称。
- 其他成员的消息归属不因成员顺序变化而改变。
- 刷新或重新打开会话后，历史发言者仍保持一致。

这是最重要的回归点：成员离席只能改变当前阵容，不能重写历史。

## 第六步：验证单 Agent 不受影响

1. 新建另一个会话。
2. 会话类型选择**单 Agent**。
3. 发送一条普通消息。

### 检查点 F：兼容性

- 不显示多成员条。
- 不显示成员切换或多 Agent 路由状态。
- 消息继续使用普通的 Agent 标签。
- 原有 Agent Terminal 行为保持不变。

## 自动化用例与检查点映射

专项测试文件是 `tests/e2e/multi-agent-session.spec.ts`：

| 自动化用例 | 覆盖内容 |
| --- | --- |
| `the multi-Agent mode is offered and composes a line-up` | 多 Agent 模式和默认席位 |
| `a seat can be added and removed before the session is created` | 创建前增删成员 |
| `a multi-seat session shows its seats and switches a seat-scoped tab` | 同会话席位展示和席位级视图 |
| `a running shared session exposes roster presence...` | 成员条、运行时增删和 `@` 补全 |
| `a single-Agent session offers no seat switcher` | 单 Agent 回归保护 |

只运行其中一个用例时，可以使用：

```powershell
npx playwright test tests/e2e/multi-agent-session.spec.ts --grep "running shared session"
```

需要查看浏览器操作过程时：

```powershell
npx playwright test tests/e2e/multi-agent-session.spec.ts --headed
```

失败后打开 trace：

```powershell
npx playwright show-trace test-results\<失败用例目录>\trace.zip
```

## 完整验证命令

准备提交前运行：

```powershell
npm run lint:ci
npm run test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
openspec validate complete-multi-agent-session-presence --strict
openspec validate --specs --strict
npx playwright test
```

## 测试记录模板

| 检查点 | 结果 | 证据或备注 |
| --- | --- | --- |
| A：创建界面 | 通过 / 失败 |  |
| B：协作房间头部 | 通过 / 失败 |  |
| C：成员管理 | 通过 / 失败 |  |
| D：交接和消息身份 | 通过 / 失败 |  |
| E：稳定历史身份 | 通过 / 失败 |  |
| F：单 Agent 兼容性 | 通过 / 失败 |  |

机制和限制的完整说明见[多 Agent 群聊与 `@` 交接](multi-agent-workflow.md)。遇到席位无法获得发言权或 `@` 未触发时，查看[故障排查](troubleshooting.md#多-agent-相关)。
