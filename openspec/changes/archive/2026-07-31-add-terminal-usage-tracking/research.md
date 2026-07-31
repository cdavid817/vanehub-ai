# 嵌入式终端 Token 统计 — 调研报告

## 四个 CLI 的会话数据持久化现状

### Claude Code

| 项目 | 值 |
|---|---|
| 会话日志路径 | `~/.claude/projects/<project-hash>/<uuid>.jsonl` |
| 文件格式 | JSONL,一行一个 stream-json 事件 |
| 关键事件类型 | `assistant`(非 `result`) |
| usage 位置 | `message.usage`(嵌套在 assistant 事件的 message 子对象里) |
| 实测数据 | `input_tokens=2, output_tokens=1959, cache_read_input_tokens=9255, cache_creation_input_tokens=20452` |
| 写入时机 | 每次 assistant 回复完成后**立即追加**到文件,tail 可见 |
| 可行评估 | ✅ 可以直接 tail session JSONL 文件,实时提取每个 assistant 回复的 usage |

### OpenCode

| 项目 | 值 |
|---|---|
| 会话存储 | `~/.local/share/opencode/opencode.db`(SQLite) |
| 其他目录 | `~/.config/opencode/`, `~/.local/state/opencode/` |
| 格式 | SQLite,非 JSONL |
| 可行评估 | ⚠️ schema 需要进一步探查,理论上可以从 SQLite 查询 |

### Codex CLI

| 项目 | 值 |
|---|---|
| 数据目录 | `~/.codex/`(全局状态 JSON + sandbox 配置) |
| 会话日志 | 未找到独立的 session JSONL 文件 |
| 可行评估 | ⚠️ 需要进一步确认是否持久化 session log |

### Gemini CLI

| 项目 | 值 |
|---|---|
| 数据目录 | `~/.gemini/projects.json`(项目元数据) |
| 会话日志 | 未找到独立的 session JSONL 文件 |
| 可行评估 | ⚠️ 需要进一步确认是否持久化 session log |

## 方案修正

### 原方案(事后日志读取)的问题

- Claude Code 的交互 session log **没有 `result` 事件**(只有 `-p --output-format stream-json` 才有)
- Usage 数据在 **`assistant` 事件的 `message.usage` 里**,不是 `result.usage`
- 每个 assistant 回复一条 usage,需要按消息粒度而非 session 粒度存储
- OpenCode 用 SQLite 而非 JSONL

### 修正后方案: Claude Code 优先,实时 JSONL tail

**原理:** Claude Code 写入 session JSONL 文件时是逐行追加的——每条 assistant 回复完成后文件立马多一行。终端进程可以在后台 tail 这个文件,实时检测新的 `assistant` 事件并提取 `message.usage`。

**路径计算:**
- `build_interactive_invocation` 为 claude-code 分配 `assigned_runtime_session_id`(UUID)
- Claude Code 用 `--session-id` 标志(同 UUID)启动
- Claude Code 的 session JSONL 路径 = `~/.claude/projects/<cwd-hash>/<session-id>.jsonl`
- `cwd-hash` 由 `request.session.folder` 决定(claude-code 对 cwd 做确定性哈希)

**挂钩点(在 terminal_process.rs 已确认):**
`PortablePtyAgentTerminalRuntime::open_or_attach()` (line 157) 里的后台 PTY reader 线程 (line 370-491) 有两处可挂:
1. PTY 输出解析循环(drain_complete_lines)中,虽然不解析 usage,但它**已经在 tail 文件**(ProviderSessionCapture 机制在做类似的事)。可以扩展现有的 `ProviderSessionCapture` 或在旁开一个独立的文件 tailer。
2. 或者直接在 reader 循环**结束后**(line 491-504),一次性从已写满的 session JSONL 文件读取所有 usage——这更简单,不需要实时。

**推荐路线: session 结束后一次性读取(更可靠)**

在 reader 循环退出(line 491)到终端清理(line 505)之间,如果是 claude-code:
1. 根据 `session.folder` 推导 cwd-hash
2. 定位 `~/.claude/projects/<hash>/<assigned_runtime_session_id>.jsonl`
3. 解析所有 `{"type":"assistant","message":{"usage":{...}}}` 行
4. 对每条含 usage 的行 → 创建合成 message → 写 usage_records
5. 前端通过 `AgentTerminalEvent::State(Stopped)` → `useEffect` → `invalidateQueries` 自动刷新

**改动范围:**
- `terminal_process.rs`: 在 reader 结束后加 30-40 行钩子逻辑
- 新增 `claude_session_log_reader.rs`(~60 行): cwd-hash 计算 + JSONL 尾行读取
- `infrastructure/sessions_gateway.rs`: 新增 `create_synthetic_message_with_usage()` 方法
- 前端 `agent-terminal-tab.tsx`: 在 `State(Stopped)` 事件回调里加 `invalidateQueries(["session-usage-summary"])`
- 总共 4 个文件,~150 行净增代码

**不支持:**
- OpenCode(需要额外探查 SQLite schema)
- Codex CLI / Gemini CLI(未找到独立 session log,留待后续调研)

### 实测数据佐证

```text
$ python -c "
with open('~/.claude/projects/D--cdavid-Documents-code-vanehub-ai/6e02d7cf-...jsonl') as f:
    lines = [json.loads(l) for l in f if l.strip()]
assistant_lines = [l for l in lines if l.get('type') == 'assistant']
non_zero = sum(1 for a in assistant_lines if a.get('message',{}).get('usage',{}).get('input_tokens',0)>0)
print(f'total assistants: {len(assistant_lines)}, with non-zero usage: {non_zero}')
# OUTPUT: total assistants: 137, with non-zero usage: 136
```
