# 交接：多 Agent 串行轮次协调器（任务 7.2）

对应变更：`openspec/changes/add-multi-agent-group-chat-session`
分支：`worktree-multi-agent-coordination-ui`（领先 `origin/main` 24 个提交）
状态：39/50 任务完成。本文档覆盖剩下的关键路径。

---

## 一句话说清现状

**角色会被注入、Agent 被告知了 `@` 规则、解析器和护栏全都写好测好了——但没有人在回复结束后去调用它们。**

所以现在创建多 Agent 会话，Agent 可能真的在行首写出 `@代码审查`，然后什么都不会发生。7.2 就是补上这个"没有人"。

一旦 7.2 完成，另外 4 个任务会同时解锁（8.3、8.4、9.5、10.3），它们都只是在等协调器提供状态。

---

## 一、架构：为什么不能从 sink 里直接链式发起

**最直觉的做法是错的。** 生成完成时 `GenerationEventHandler::completed()` 手里已经有完整回复文本了，看起来在那里解析 `@` 并发起下一轮最自然。不要这么做：

1. `GenerationEventHandler` 持有的是 `ports`，**不是 application service**。发起一次生成是 service 级操作，sink 够不着。
2. 在终态处理器里发起新生成，会让生成的生命周期互相嵌套——上一轮还没走完终态流程，下一轮已经开始。

**循环工程已经解决过一模一样的形状**，照抄它：

```
生成完成
   │
   ▼
GenerationEventHandler          ← 不做决策，只投递
   │  ports.loop_completions.deliver(LoopRoleGenerationTerminal { ... })
   ▼
独立协调器（loop scheduler）      ← 在这里决定下一步、发起下一次生成
```

先例的准确位置：

| 东西 | 位置 |
|---|---|
| 端口定义 | `src-tauri/src/contexts/agent_runtime/application/ports.rs:554` `LoopRoleGenerationCompletionPort` |
| 终态类型 | `LoopRoleGenerationTerminal` |
| sink 侧投递 | `application/service.rs:2664` 附近的 `deliver_loop_terminal` |
| sink 上的所有权标记 | `GenerationEventHandler.loop_ownership`（`service.rs:2177`） |
| 消费方 | `bootstrap/agent_runtime.rs`、`application/service.rs` |

注意端口是**双向**的：`deliver(terminal)` 投递，`take_for_session(session_id)` 取走。座位轮次大概率需要同样的形状。

---

## 二、已经建好的零件（都已测试，直接调用即可）

协调器**只负责驱动**，所有判断逻辑都已就绪：

| 文件 | 提供什么 | 测试数 |
|---|---|---|
| `src/services/mention-routing.ts` | `parseHandoffMentions` — 行首 `@` 解析，含全部护栏 | 12 |
| `src/services/turn-routing.ts` | `nextTurnTargets`（链深上限 + 截断原因）、`routeUserMessage`（无 `@` 时投给上一持球者） | 7 |
| `src/services/human-handoff.ts` | `parseHumanHandoff`、`applyHumanHandoff` — 三态 intent 及其对轮次的影响 | 9 |
| `src/services/seat-context.ts` | `buildSeatContext` — resume 优先，否则按预算注入前序发言 | 4 |
| `src/services/seat-briefing.ts` | `buildSeatBriefing` — 角色正文 + 名单 + 交接规则 | 6 |
| `src/services/role-injection-channel.ts` | `roleInjectionChannel` — 哪些 CLI 有原生 system 通道 | 2 |
| `src/services/seat-mutation.ts` | `addSeat` / `removeSeat` — 含"最后一席不可删""删首席要重镜像 agentId" | 7 |
| `src/services/message-speaker.ts` | `resolveMessageSpeaker` — 席位→可渲染身份 | 5 |

**注入侧已经打通**：`providers/invocation.rs` 的 `build_invocation_with_role` 已接进 `process_adapter.rs`，`GenerationProcessRequest.role_briefing` 字段已存在。协调器只需在发起下一席位的生成时**填上这个字段**（目前唯一的生产构造点 `application/service.rs:1730` 附近传的是 `None`）。

⚠️ 上述逻辑目前都在 **TypeScript** 里。协调器在 Rust 侧，需要决定是移植还是把决策留在前端——**这是开工前第一个要定的问题**，见第四节。

---

## 三、建议的实施步骤

1. **决定决策逻辑放哪**（见第四节的开放问题）。
2. 在 `GenerationEventHandler` 上加 `seat_ownership: Option<SeatTurnOwnership>`，与 `loop_ownership` 并列。构造点在 `service.rs:2269` 附近。
3. 定义 `SeatTurnTerminal`（会话 id、席位序号、完整回复文本、链深）与 `SeatTurnCompletionPort`，照 `LoopRoleGenerationCompletionPort` 的形状。
4. 在 `completed()` 成功路径投递终态——**不要在 `complete_claimed` 里投**，等消息真正落库后再投，否则协调器可能读到尚未持久化的回复。
5. 写协调器：解析 `@` → 应用链深/条数上限 → 若命中人类 intent 则按 `applyHumanHandoff` 决定是否停 → 否则为目标席位发起生成，带上 `role_briefing` 和 `buildSeatContext` 的产物。
6. 接进 `bootstrap/agent_runtime.rs`，位置在 loop scheduler 旁边。
7. 解锁 8.3 / 8.4 / 9.5 / 10.3。

---

## 四、开工前要定的两个问题

**一、决策逻辑放前端还是 Rust？**
所有解析和路由决策现在是 TypeScript 且有 75 个测试。选项：(a) 移植到 Rust，测试重写一遍；(b) 协调器只做调度，把"下一个是谁"的决策留在前端，通过事件往返。(a) 更内聚但要重写测试，(b) 省事但每轮多一次往返、且 Agent 已经跑完了才问前端。**我倾向 (a)**，但没有实证依据，值得先想清楚。

**二、`@` 到底可不可靠？**
design.md 的 Risks 里明确标了：**跨 CLI 厂商，Agent 是否会稳定地在行首输出 `@`，这件事是真的不确定**。任务 11.6 专门留了一条"拿真实 Agent 验证；不成立就先改 roster 措辞，别急着扩范围"。

**建议在写协调器之前先做 11.6。** 用真实的 claude-code / codex-cli 各跑一次，把 `buildSeatBriefing` 的产物当 system prompt 喂进去，看它们会不会照规则输出行首 `@`。如果不会，协调器再完美也没用——该改的是 briefing 措辞。这个验证半小时能出结果，能省掉可能白写的一天。

---

## 五、这次会话踩过的坑（别重踩）

**`git add <不存在的路径>` 会整体失败、什么都不暂存。** 我曾用 `2>/dev/null` 吞掉错误，导致三个改动文件从未进入提交，随后又被 `git checkout --` 还原。**每次提交前看 `git diff --cached --numstat` 确认非空**。

**Python 的 `str.replace` 找不到锚点时静默返回原文。** 踩过两次。批量改代码时加 `assert anchor in s`。

**`npm run test` 不做类型检查。** 四个测试调用点缺参数，vitest 全绿、`npm run build`（走 tsc）才报错。**改了类型必须跑 build**。

**迁移号不能只看源码里最大的字面量。** 这台机器上多个 worktree 共用同一个 `ai.vanehub.app` 数据库，未合并分支的迁移已经记录在里面。查真实的 `schema_migrations`：
```
python -c "import sqlite3;c=sqlite3.connect(r'C:/Users/cdavid/AppData/Roaming/ai.vanehub.app/vanehub.sqlite');print(c.execute('SELECT MAX(version) FROM schema_migrations').fetchone())"
```
沿用已被占用的号，迁移会**永远被跳过**。当前源码用到 45。

**pnpm 污染在本次会话发生了四次。** 症状是 `npm run build` 挂在 `runtime-floating-assistant-client` 的 chunk 预算上——报的 chunk 与真实原因无关。检测用 `ls node_modules/.pnpm`（**不要**看 `pnpm-lock.yaml`，它在至少一次污染中并不存在），修复用 `npm ci`。

**relay 测试同批跑时会偶发失败。** `contexts::tooling::mcp::infrastructure::relay::tests::*` 是 socket 时序抖动，单独跑必过。别当回归。

**Playwright 会连上别的 worktree 的 dev server。** `playwright.config.ts` 是 `reuseExistingServer: true` + 写死 5174，别的 worktree 占了这个端口就会**静默地对着错误的代码库跑整套 E2E**。自己起服务并 `PLAYWRIGHT_PORT=<你的端口>`，再配 `env -u all_proxy -u ALL_PROXY`。

---

## 六、验证

```bash
npm ci                       # 先确认 node_modules/.pnpm 不存在
npm run lint && npm run test && npm run build
cargo clippy --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npx openspec validate add-multi-agent-group-chat-session --strict
```

当前基线：前端 **614/614**、原生 **1245/1245**、clippy **0 告警**、规范校验通过。

---

## 七、完成后

剩余任务：8.3、8.4、9.5、10.3（都随协调器解锁）、11.1–11.6（验证，含 11.6 的真实 Agent 验证）。

全部完成后走 `openspec archive add-multi-agent-group-chat-session` 加 `scripts/Update-OpenSpecArchiveIndex.ps1`。
