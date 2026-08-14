# OnePiece 斜杠命令（第一阶段）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 OnePiece 会话的聊天输入框中提供应用级斜杠命令：输入 `/xxx` 由前端拦截执行，不发送给模型。本阶段只做零后端改动的命令。

**Architecture:** 命令注册表与解析器是无依赖的纯函数模块，位于 `src/services/slash-commands/`。拦截发生在 `api-session-composer.tsx`（而非 `use-main-layout-model.ts`，后者是 298 行、无 `max-lines` 豁免）。命令输出存于独立 React state，渲染成输入框上方的临时面板，不进入消息流。作用域由单一谓词模块 `command-availability.ts` 门控。

**Tech Stack:** React 19 + TypeScript strict、Tailwind、Vitest + Testing Library、react-i18next。

**设计依据:** `docs/superpowers/specs/2026-08-14-onepiece-slash-commands-design.md`

## Global Constraints

- 单文件不超过 300 物理行（`max-lines` 为 ESLint error）。禁止向 `eslint.config.js` 的豁免清单新增文件。测试文件豁免
- 禁止 `any`，禁止 `// @ts-ignore`（需绕过时用 `// @ts-expect-error` 并写明原因）
- 函数组件 + Hooks，禁止 class component
- 样式只用 Tailwind 工具类，禁止内联 style，禁止引入新 UI 库
- 组件禁止直接调用 Tauri `invoke()`，必须经 `src/services/` 边界
- 注释只写"为什么这样做"，不写代码翻译式注释
- i18n locale 文件使用**扁平点号键**（如 `"chat.placeholder": "..."`），不是嵌套对象。五个 locale 必须同步：`en` / `ja` / `ko` / `zh-CN` / `zh-TW`
- 测试用 `renderWithAppProviders`（`src/test/render.tsx`），它挂载真实 i18n；缺 key 时 `t()` 返回 key 本身，因此 key 齐全性由 parity 测试保证
- 提交信息用英文，遵循 Conventional Commits，body 每行不超过 100 字符
- **本阶段不实现** `/clear` 与 `/compact`——它们需要新的后端能力，属于第二阶段

## 已知类型（照抄，勿重新发明）

```ts
// src/types/agent.ts
type InteractionMode = "browser" | "native-desktop" | "cli" | "api";
type SessionExportFormat = "json" | "markdown";
interface SessionSeat { agentId: string; roleId: string | null; leftAt?: string | null }
interface Session { id: string; title: string; agentId: string; seats?: SessionSeat[];
                    interactionMode: InteractionMode; /* …其余字段本计划不用… */ }

// src/types/chat.ts
type ReasoningDepth = "low" | "medium" | "high" | "max";
type SessionExecutionMode = "inherit" | "plan" | "execute";
interface ChatConfig { agentId: string; interactionMode: InteractionMode;
  executionMode: SessionExecutionMode; providerId?: string; modelId?: string;
  reasoningDepth?: ReasoningDepth; streaming: boolean; thinking: boolean; longContext: boolean }
interface ReportedTokenTotals { inputTokens: number; outputTokens: number;
  cacheReadTokens: number; cacheCreationTokens: number; totalTokens: number }
interface SessionUsageSummary { sessionId: string; reported: ReportedTokenTotals;
  responseCount: number; generatedAt: string; /* …其余字段本计划不用… */ }

// src/session-workspace/session-tab-bar.ts
type SessionTabId = "chat" | "changes" | "documents" | "files" | "terminal"
                  | "shell" | "logs" | "traces" | "report";
```

## File Structure

| 文件 | 职责 |
| --- | --- |
| `src/services/slash-commands/parse-command.ts` | 解析输入为 message / literal / command，纯函数 |
| `src/services/slash-commands/command-availability.ts` | 会话类型谓词，唯一门控真源 |
| `src/services/slash-commands/types.ts` | `SlashCommand`、`CommandContext`、`CommandOutcome` 等共享类型 |
| `src/services/slash-commands/command-registry.ts` | 对命令数组的纯查找函数 |
| `src/services/slash-commands/runtime-commands.ts` | `/mode` `/reasoning` `/thinking` `/streaming` `/longcontext` |
| `src/services/slash-commands/session-commands.ts` | `/export` `/stop` `/status` `/usage` |
| `src/services/slash-commands/navigation-commands.ts` | `/plan` `/plans` `/loops` `/todo` 与工作区页签 |
| `src/services/slash-commands/help-command.ts` | `/help` |
| `src/services/slash-commands/command-catalog.ts` | 汇总全部命令为 `SLASH_COMMANDS` |
| `src/services/slash-commands/use-slash-commands.ts` | hook：输出面板 state + `dispatch` |
| `src/components/chat/SlashCommandCompletion.tsx` | `/` 补全下拉 |
| `src/components/chat/SlashCommandOutput.tsx` | 输入框上方的临时输出面板 |

按职责而非技术分层切分：命令定义按类别分文件，是为了让每个文件都远低于 300 行且能被独立评审。

---

### Task 1: 输入解析器

**Files:**
- Create: `src/services/slash-commands/parse-command.ts`
- Test: `src/services/slash-commands/parse-command.test.ts`

**Interfaces:**
- Consumes: 无
- Produces: `parseCommandInput(draft: string): ParsedInput`，其中
  `ParsedInput = { kind: "message" } | { kind: "literal"; content: string } | { kind: "command"; name: string; args: string[] }`

命令形态限定为单行、`/` 后紧跟小写字母开头的名字。这样 `/usr/bin/env`、多行粘贴的代码块都不会被误判成命令。`//` 前缀转义为字面 `/`，让用户仍能发送真正以斜杠开头的文本。

- [ ] **Step 1: Write the failing test**

创建 `src/services/slash-commands/parse-command.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { parseCommandInput } from "./parse-command";

describe("parseCommandInput", () => {
  it("treats ordinary prose as a message", () => {
    expect(parseCommandInput("hello world")).toEqual({ kind: "message" });
    expect(parseCommandInput("")).toEqual({ kind: "message" });
    expect(parseCommandInput("   ")).toEqual({ kind: "message" });
  });

  it("parses a bare command and lowercases the name", () => {
    expect(parseCommandInput("/help")).toEqual({ kind: "command", name: "help", args: [] });
    expect(parseCommandInput("  /Help  ")).toEqual({ kind: "command", name: "help", args: [] });
  });

  it("splits arguments on runs of whitespace", () => {
    expect(parseCommandInput("/mode   plan")).toEqual({ kind: "command", name: "mode", args: ["plan"] });
    expect(parseCommandInput("/export json extra")).toEqual({ kind: "command", name: "export", args: ["json", "extra"] });
  });

  it("unescapes a doubled slash into literal text", () => {
    expect(parseCommandInput("//help")).toEqual({ kind: "literal", content: "/help" });
    expect(parseCommandInput("//usr/bin/env python")).toEqual({ kind: "literal", content: "/usr/bin/env python" });
  });

  it("leaves paths and multi-line input alone", () => {
    expect(parseCommandInput("/usr/bin/env")).toEqual({ kind: "message" });
    expect(parseCommandInput("/help\nsecond line")).toEqual({ kind: "message" });
    expect(parseCommandInput("/1234")).toEqual({ kind: "message" });
    expect(parseCommandInput("/")).toEqual({ kind: "message" });
  });

  it("recognises the prefix a completion dropdown should react to", () => {
    expect(parseCommandInput("/mod")).toEqual({ kind: "command", name: "mod", args: [] });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/services/slash-commands/parse-command.test.ts`
Expected: FAIL — `Failed to resolve import "./parse-command"`

- [ ] **Step 3: Write the implementation**

创建 `src/services/slash-commands/parse-command.ts`：

```ts
export type ParsedInput =
  | { kind: "message" }
  | { kind: "literal"; content: string }
  | { kind: "command"; name: string; args: string[] };

// A command name must start with a letter and the whole draft must be one line. Without those
// two guards `/usr/bin/env` and pasted code blocks would be swallowed as unknown commands.
const COMMAND_PATTERN = /^\/([a-zA-Z][a-zA-Z0-9-]*)(?:\s+(.*))?$/;

export function parseCommandInput(draft: string): ParsedInput {
  const trimmed = draft.trim();
  if (trimmed.includes("\n")) return { kind: "message" };
  // `//` is the escape hatch for prose that genuinely starts with a slash; unknown commands are
  // rejected rather than forwarded, so without it such prose would be unsendable.
  if (trimmed.startsWith("//")) return { kind: "literal", content: trimmed.slice(1) };

  const match = COMMAND_PATTERN.exec(trimmed);
  if (!match) return { kind: "message" };

  const args = (match[2] ?? "").split(/\s+/).filter((part) => part.length > 0);
  return { kind: "command", name: match[1].toLowerCase(), args };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/services/slash-commands/parse-command.test.ts`
Expected: PASS — 6 tests

- [ ] **Step 5: Commit**

```bash
git add src/services/slash-commands/parse-command.ts src/services/slash-commands/parse-command.test.ts
git commit -m "feat: add slash command input parser"
```

---

### Task 2: 会话可用性谓词

**Files:**
- Create: `src/services/slash-commands/command-availability.ts`
- Test: `src/services/slash-commands/command-availability.test.ts`

**Interfaces:**
- Consumes: `activeSeatsFromSession` from `src/services/session-seats.ts`
- Produces: `isOnePieceSession(session: Session): boolean`、`isMultiSeatCliSession(session: Session): boolean`、`slashCommandsEnabled(session: Session | null): boolean`

这是唯一的门控真源。第二阶段之后要放开多席位 CLI 会话，只改 `slashCommandsEnabled` 一处。

- [ ] **Step 1: Write the failing test**

创建 `src/services/slash-commands/command-availability.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import type { Session } from "../../types/agent";
import { isMultiSeatCliSession, isOnePieceSession, slashCommandsEnabled } from "./command-availability";

const session = (overrides: Partial<Session>): Session => ({
  id: "session-1", title: "Session", agentId: "onepiece", interactionMode: "api",
  lifecycleState: "idle", recoveryStatus: "healthy", recoveryRevision: 0, stateRevision: 0,
  historyRevision: 0, activeExecutionRunId: null, folder: null, projectPath: null,
  worktreePath: null, worktreeName: null, worktreeBranch: null, remoteWorkspace: null,
  remoteSshConnectionId: null, remoteSshConnectionRevision: null, runtimeSessionId: null,
  categoryId: null, pinned: false, archived: false,
  createdAt: "2026-08-14T00:00:00Z", updatedAt: "2026-08-14T00:00:00Z",
  ...overrides,
} as Session);

describe("slash command availability", () => {
  it("recognises a OnePiece session", () => {
    expect(isOnePieceSession(session({ agentId: "onepiece" }))).toBe(true);
    expect(isOnePieceSession(session({ agentId: "claude-code" }))).toBe(false);
  });

  it("recognises a multi-seat CLI session", () => {
    const seats = [{ agentId: "claude-code", roleId: null }, { agentId: "codex-cli", roleId: null }];
    expect(isMultiSeatCliSession(session({ agentId: "claude-code", interactionMode: "cli", seats }))).toBe(true);
    expect(isMultiSeatCliSession(session({ agentId: "claude-code", interactionMode: "cli" }))).toBe(false);
    expect(isMultiSeatCliSession(session({ agentId: "onepiece", interactionMode: "api", seats }))).toBe(false);
  });

  it("ignores seats that have already left", () => {
    const seats = [
      { agentId: "claude-code", roleId: null },
      { agentId: "codex-cli", roleId: null, leftAt: "2026-08-14T00:00:00Z" },
    ];
    expect(isMultiSeatCliSession(session({ interactionMode: "cli", seats }))).toBe(false);
  });

  it("enables commands for OnePiece only in this phase", () => {
    const seats = [{ agentId: "claude-code", roleId: null }, { agentId: "codex-cli", roleId: null }];
    expect(slashCommandsEnabled(session({ agentId: "onepiece" }))).toBe(true);
    expect(slashCommandsEnabled(session({ agentId: "claude-code", interactionMode: "cli", seats }))).toBe(false);
    expect(slashCommandsEnabled(null)).toBe(false);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/services/slash-commands/command-availability.test.ts`
Expected: FAIL — `Failed to resolve import "./command-availability"`

- [ ] **Step 3: Write the implementation**

创建 `src/services/slash-commands/command-availability.ts`：

```ts
import { activeSeatsFromSession } from "../session-seats";
import type { Session } from "../../types/agent";

/** `agentId === "onepiece"` is the codebase's established test for the built-in native agent. */
export function isOnePieceSession(session: Session): boolean {
  return session.agentId === "onepiece";
}

export function isMultiSeatCliSession(session: Session): boolean {
  return session.interactionMode === "cli" && activeSeatsFromSession(session).length > 1;
}

/**
 * The single gate. Phase 1 ships OnePiece only; widening to multi-seat CLI sessions later is a
 * change to this one expression plus each command's own `appliesTo`, and touches nothing else.
 */
export function slashCommandsEnabled(session: Session | null): boolean {
  if (!session) return false;
  return isOnePieceSession(session);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/services/slash-commands/command-availability.test.ts`
Expected: PASS — 4 tests

- [ ] **Step 5: Commit**

```bash
git add src/services/slash-commands/command-availability.ts src/services/slash-commands/command-availability.test.ts
git commit -m "feat: add slash command availability predicates"
```

---

### Task 3: 命令类型与注册表查找

**Files:**
- Create: `src/services/slash-commands/types.ts`
- Create: `src/services/slash-commands/command-registry.ts`
- Test: `src/services/slash-commands/command-registry.test.ts`

**Interfaces:**
- Consumes: `slashCommandsEnabled` (Task 2)
- Produces: 类型 `SlashCommand`、`CommandContext`、`CommandOutcome`、`CommandOutput`、`CommandMessage`、`SlashCommandDestination`；函数 `findCommand(commands, name, session)`、`listCommands(commands, session)`

命令输出用**翻译 key + 参数**而非成品字符串，这样命令的单元测试不依赖 i18n，断言的是 key。翻译发生在 `SlashCommandOutput` 组件里。

- [ ] **Step 1: Write the failing test**

创建 `src/services/slash-commands/command-registry.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import type { Session } from "../../types/agent";
import { findCommand, listCommands } from "./command-registry";
import type { SlashCommand } from "./types";

const session = (agentId: string): Session =>
  ({ id: "s", title: "S", agentId, interactionMode: "api" } as Session);

const command = (name: string, overrides: Partial<SlashCommand> = {}): SlashCommand => ({
  name, category: "info", appliesTo: () => true,
  run: async () => ({ kind: "handled" }), ...overrides,
});

const capabilities = (hasAssociatedPlan = false) => ({ hasAssociatedPlan });

describe("command registry", () => {
  const commands = [
    command("help"),
    command("status", { aliases: ["st"] }),
    command("compact", { appliesTo: (target) => target.agentId === "onepiece" }),
    command("plan", { appliesTo: (_target, caps) => caps.hasAssociatedPlan }),
  ];

  it("finds a command by name", () => {
    expect(findCommand(commands, "help", session("onepiece"), capabilities())?.name).toBe("help");
  });

  it("finds a command by alias", () => {
    expect(findCommand(commands, "st", session("onepiece"), capabilities())?.name).toBe("status");
  });

  it("returns null for an unknown name", () => {
    expect(findCommand(commands, "nope", session("onepiece"), capabilities())).toBeNull();
  });

  it("returns null when the command does not apply to the session", () => {
    expect(findCommand(commands, "compact", session("claude-code"), capabilities())).toBeNull();
    expect(findCommand(commands, "compact", session("onepiece"), capabilities())?.name).toBe("compact");
  });

  it("consults capabilities, not just the session", () => {
    expect(findCommand(commands, "plan", session("onepiece"), capabilities(false))).toBeNull();
    expect(findCommand(commands, "plan", session("onepiece"), capabilities(true))?.name).toBe("plan");
  });

  it("lists only applicable commands, sorted by name", () => {
    expect(listCommands(commands, session("claude-code"), capabilities()).map((entry) => entry.name)).toEqual(["help", "status"]);
    expect(listCommands(commands, session("onepiece"), capabilities(true)).map((entry) => entry.name)).toEqual(["compact", "help", "plan", "status"]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/services/slash-commands/command-registry.test.ts`
Expected: FAIL — `Failed to resolve import "./command-registry"`

- [ ] **Step 3: Write the type module**

创建 `src/services/slash-commands/types.ts`：

```ts
import type { Session, SessionExportFormat } from "../../types/agent";
import type { ChatConfig, ReasoningDepth, SessionExecutionMode } from "../../types/chat";
import type { SessionTabId } from "../../session-workspace/session-tab-bar";

export type SlashCommandCategory = "session" | "runtime" | "navigation" | "info";

export type SlashCommandDestination = "sessions" | "loops" | "plans" | "todo-board";

/**
 * Facts a command needs for its availability decision that do not live on the session row.
 * Passing them explicitly is what keeps `appliesTo` a pure function of its arguments.
 */
export interface CommandCapabilities {
  hasAssociatedPlan: boolean;
}

/**
 * Commands emit translation keys rather than finished strings so their unit tests stay free of
 * i18n, and so the output panel remains the single place that renders copy.
 */
export interface CommandMessage {
  key: string;
  params?: Record<string, string | number>;
}

export interface CommandOutput {
  titleKey: string;
  messages: CommandMessage[];
  tone: "info" | "error";
}

export type CommandOutcome =
  | { kind: "handled" }
  | { kind: "output"; output: CommandOutput };

export interface SlashCommandNavigation {
  /** Null when the session has no associated plan run, which makes `/plan` inapplicable. */
  openAssociatedPlan: (() => void) | null;
  openDestination: (destination: SlashCommandDestination) => void;
  openSessionTab: (tab: SessionTabId) => void;
}

export interface CommandContext {
  session: Session;
  config: ChatConfig;
  isStreaming: boolean;
  chat: {
    setSessionExecutionMode: (value: SessionExecutionMode) => void;
    setReasoningDepth: (value: ReasoningDepth) => void;
    setStreaming: (value: boolean) => void;
    setThinking: (value: boolean) => void;
    setLongContext: (value: boolean) => void;
  };
  actions: {
    exportSession: (session: Session, format: SessionExportFormat) => void;
    stop: () => void;
    loadUsageSummary: (sessionId: string) => Promise<{
      totalTokens: number; inputTokens: number; outputTokens: number; responseCount: number;
    }>;
  };
  navigate: SlashCommandNavigation;
  /** Supplied by the dispatcher so `/help` can enumerate siblings without a circular import. */
  listAvailableCommands: () => SlashCommand[];
}

export interface SlashCommand {
  name: string;
  aliases?: string[];
  category: SlashCommandCategory;
  /** Rendered in `/help` and the completion dropdown, e.g. "<plan|execute|inherit>". */
  argumentHint?: string;
  /** Commands that only care about the session may declare a one-parameter function. */
  appliesTo: (session: Session, capabilities: CommandCapabilities) => boolean;
  run: (context: CommandContext, args: string[]) => Promise<CommandOutcome>;
}
```

- [ ] **Step 4: Write the registry**

创建 `src/services/slash-commands/command-registry.ts`：

```ts
import type { Session } from "../../types/agent";
import type { CommandCapabilities, SlashCommand } from "./types";

function matches(command: SlashCommand, name: string): boolean {
  return command.name === name || (command.aliases?.includes(name) ?? false);
}

export function findCommand(
  commands: SlashCommand[], name: string, session: Session, capabilities: CommandCapabilities,
): SlashCommand | null {
  const command = commands.find((entry) => matches(entry, name));
  if (!command || !command.appliesTo(session, capabilities)) return null;
  return command;
}

export function listCommands(
  commands: SlashCommand[], session: Session, capabilities: CommandCapabilities,
): SlashCommand[] {
  return commands
    .filter((command) => command.appliesTo(session, capabilities))
    .sort((left, right) => left.name.localeCompare(right.name));
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `npx vitest run src/services/slash-commands/command-registry.test.ts`
Expected: PASS — 5 tests

- [ ] **Step 6: Commit**

```bash
git add src/services/slash-commands/types.ts src/services/slash-commands/command-registry.ts src/services/slash-commands/command-registry.test.ts
git commit -m "feat: add slash command types and registry lookup"
```

---

### Task 4: 运行时切换命令

**Files:**
- Create: `src/services/slash-commands/runtime-commands.ts`
- Create: `src/services/slash-commands/command-catalog.ts`
- Test: `src/services/slash-commands/runtime-commands.test.ts`
- Modify: `src/i18n/locales/en.json`、`ja.json`、`ko.json`、`zh-CN.json`、`zh-TW.json`
- Test: `src/i18n/slash-command-locales.test.ts`

**Interfaces:**
- Consumes: `SlashCommand`、`CommandContext`、`CommandOutcome`（Task 3）、`isOnePieceSession`（Task 2）
- Produces: `RUNTIME_COMMANDS: SlashCommand[]`、`SLASH_COMMANDS: SlashCommand[]`

`/model`、`/provider`、`/agent` **不实现**：`api-session-composer.tsx:23` 无条件传 `lockRuntimeIdentity`，`ButtonArea.tsx:79,104` 据此禁用了 ProviderSelect 与 ModelSelect。命令绕开这个锁就是缺陷。

- [ ] **Step 1: Write the failing test**

创建 `src/services/slash-commands/runtime-commands.test.ts`：

```ts
import { describe, expect, it, vi } from "vitest";
import type { Session } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import { RUNTIME_COMMANDS } from "./runtime-commands";
import type { CommandContext, SlashCommand } from "./types";

const session = (agentId = "onepiece"): Session =>
  ({ id: "s", title: "S", agentId, interactionMode: "api" } as Session);

const config = (overrides: Partial<ChatConfig> = {}): ChatConfig => ({
  agentId: "onepiece", interactionMode: "api", executionMode: "inherit",
  streaming: true, thinking: false, longContext: false, ...overrides,
});

function context(overrides: Partial<ChatConfig> = {}) {
  const chat = {
    setSessionExecutionMode: vi.fn(), setReasoningDepth: vi.fn(),
    setStreaming: vi.fn(), setThinking: vi.fn(), setLongContext: vi.fn(),
  };
  const ctx = {
    session: session(), config: config(overrides), isStreaming: false, chat,
    actions: { exportSession: vi.fn(), stop: vi.fn(), loadUsageSummary: vi.fn() },
    navigate: { openAssociatedPlan: null, openDestination: vi.fn(), openSessionTab: vi.fn() },
    listAvailableCommands: () => [],
  } as unknown as CommandContext;
  return { ctx, chat };
}

const byName = (name: string): SlashCommand => {
  const command = RUNTIME_COMMANDS.find((entry) => entry.name === name);
  if (!command) throw new Error(`missing command: ${name}`);
  return command;
};

describe("runtime commands", () => {
  it("applies only to OnePiece sessions in this phase", () => {
    // Runtime commands never read capabilities, but appliesTo's signature requires it
    // (types.ts), so every call site — including this generic sweep — must supply one.
    const capabilities = { hasAssociatedPlan: false };
    for (const command of RUNTIME_COMMANDS) {
      expect(command.appliesTo(session("onepiece"), capabilities)).toBe(true);
      expect(command.appliesTo(session("claude-code"), capabilities)).toBe(false);
    }
  });

  it("does not expose model, provider or agent switching", () => {
    const names = RUNTIME_COMMANDS.map((command) => command.name);
    expect(names).not.toContain("model");
    expect(names).not.toContain("provider");
    expect(names).not.toContain("agent");
  });

  it("/mode sets a valid execution mode and reports it", async () => {
    const { ctx, chat } = context();
    const outcome = await byName("mode").run(ctx, ["plan"]);
    expect(chat.setSessionExecutionMode).toHaveBeenCalledWith("plan");
    expect(outcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.output.applied", tone: "info",
        messages: [{ key: "slash.output.mode", params: { value: "plan" } }] },
    });
  });

  it("/mode rejects an unknown value without touching config", async () => {
    const { ctx, chat } = context();
    const outcome = await byName("mode").run(ctx, ["nonsense"]);
    expect(chat.setSessionExecutionMode).not.toHaveBeenCalled();
    expect(outcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.error.title", tone: "error",
        messages: [{ key: "slash.error.badArgument", params: { command: "mode", allowed: "inherit, plan, execute" } }] },
    });
  });

  it("/reasoning accepts every supported depth", async () => {
    for (const depth of ["low", "medium", "high", "max"]) {
      const { ctx, chat } = context();
      await byName("reasoning").run(ctx, [depth]);
      expect(chat.setReasoningDepth).toHaveBeenCalledWith(depth);
    }
  });

  it("/thinking toggles when given no argument", async () => {
    const { ctx, chat } = context({ thinking: false });
    await byName("thinking").run(ctx, []);
    expect(chat.setThinking).toHaveBeenCalledWith(true);
  });

  it("/thinking honours an explicit on or off", async () => {
    const enabled = context({ thinking: true });
    await byName("thinking").run(enabled.ctx, ["on"]);
    expect(enabled.chat.setThinking).toHaveBeenCalledWith(true);

    const disabled = context({ thinking: true });
    await byName("thinking").run(disabled.ctx, ["off"]);
    expect(disabled.chat.setThinking).toHaveBeenCalledWith(false);
  });

  it("/streaming and /longcontext toggle their own switches", async () => {
    const streaming = context({ streaming: true });
    await byName("streaming").run(streaming.ctx, []);
    expect(streaming.chat.setStreaming).toHaveBeenCalledWith(false);

    const longContext = context({ longContext: false });
    await byName("longcontext").run(longContext.ctx, []);
    expect(longContext.chat.setLongContext).toHaveBeenCalledWith(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/services/slash-commands/runtime-commands.test.ts`
Expected: FAIL — `Failed to resolve import "./runtime-commands"`

- [ ] **Step 3: Write the implementation**

创建 `src/services/slash-commands/runtime-commands.ts`：

```ts
import { isOnePieceSession } from "./command-availability";
import type { ReasoningDepth, SessionExecutionMode } from "../../types/chat";
import type { CommandContext, CommandOutcome, SlashCommand } from "./types";

const EXECUTION_MODES: SessionExecutionMode[] = ["inherit", "plan", "execute"];
const REASONING_DEPTHS: ReasoningDepth[] = ["low", "medium", "high", "max"];

function applied(key: string, value: string | number): CommandOutcome {
  return { kind: "output", output: { titleKey: "slash.output.applied", tone: "info", messages: [{ key, params: { value } }] } };
}

function badArgument(command: string, allowed: string[]): CommandOutcome {
  return {
    kind: "output",
    output: {
      titleKey: "slash.error.title", tone: "error",
      messages: [{ key: "slash.error.badArgument", params: { command, allowed: allowed.join(", ") } }],
    },
  };
}

/** No argument means "flip it", which is what a bare `/thinking` reads as. */
function resolveToggle(args: string[], current: boolean): boolean | null {
  if (args.length === 0) return !current;
  if (args[0] === "on") return true;
  if (args[0] === "off") return false;
  return null;
}

function toggleCommand(
  name: string,
  read: (context: CommandContext) => boolean,
  write: (context: CommandContext, value: boolean) => void,
  outputKey: string,
): SlashCommand {
  return {
    name, category: "runtime", argumentHint: "[on|off]", appliesTo: isOnePieceSession,
    run: async (context, args) => {
      const next = resolveToggle(args, read(context));
      if (next === null) return badArgument(name, ["on", "off"]);
      write(context, next);
      return applied(outputKey, next ? "on" : "off");
    },
  };
}

export const RUNTIME_COMMANDS: SlashCommand[] = [
  {
    name: "mode", category: "runtime", argumentHint: "<inherit|plan|execute>", appliesTo: isOnePieceSession,
    run: async (context, args) => {
      const value = args[0] as SessionExecutionMode | undefined;
      if (!value || !EXECUTION_MODES.includes(value)) return badArgument("mode", EXECUTION_MODES);
      context.chat.setSessionExecutionMode(value);
      return applied("slash.output.mode", value);
    },
  },
  {
    name: "reasoning", category: "runtime", argumentHint: "<low|medium|high|max>", appliesTo: isOnePieceSession,
    run: async (context, args) => {
      const value = args[0] as ReasoningDepth | undefined;
      if (!value || !REASONING_DEPTHS.includes(value)) return badArgument("reasoning", REASONING_DEPTHS);
      context.chat.setReasoningDepth(value);
      return applied("slash.output.reasoning", value);
    },
  },
  toggleCommand("thinking", (context) => context.config.thinking, (context, value) => context.chat.setThinking(value), "slash.output.thinking"),
  toggleCommand("streaming", (context) => context.config.streaming, (context, value) => context.chat.setStreaming(value), "slash.output.streaming"),
  toggleCommand("longcontext", (context) => context.config.longContext, (context, value) => context.chat.setLongContext(value), "slash.output.longcontext"),
];
```

- [ ] **Step 4: Create the catalog**

创建 `src/services/slash-commands/command-catalog.ts`：

```ts
import { RUNTIME_COMMANDS } from "./runtime-commands";
import type { SlashCommand } from "./types";

export const SLASH_COMMANDS: SlashCommand[] = [...RUNTIME_COMMANDS];
```

- [ ] **Step 5: Run test to verify it passes**

Run: `npx vitest run src/services/slash-commands/runtime-commands.test.ts`
Expected: PASS — 8 tests

- [ ] **Step 6: Add the i18n keys**

在五个 locale 文件中各加入下列键。文件是**扁平点号键**的 JSON 对象，追加到末尾（注意保持前一行的逗号）。

`src/i18n/locales/en.json`：

```json
"slash.output.applied": "Applied",
"slash.output.mode": "Execution mode: {{value}}",
"slash.output.reasoning": "Reasoning depth: {{value}}",
"slash.output.thinking": "Thinking: {{value}}",
"slash.output.streaming": "Streaming: {{value}}",
"slash.output.longcontext": "Long context: {{value}}",
"slash.error.title": "Command failed",
"slash.error.badArgument": "/{{command}} expects one of: {{allowed}}"
```

`src/i18n/locales/zh-CN.json`：

```json
"slash.output.applied": "已应用",
"slash.output.mode": "执行模式：{{value}}",
"slash.output.reasoning": "推理深度：{{value}}",
"slash.output.thinking": "思考：{{value}}",
"slash.output.streaming": "流式输出：{{value}}",
"slash.output.longcontext": "长上下文：{{value}}",
"slash.error.title": "命令执行失败",
"slash.error.badArgument": "/{{command}} 需要下列取值之一：{{allowed}}"
```

`src/i18n/locales/zh-TW.json`：

```json
"slash.output.applied": "已套用",
"slash.output.mode": "執行模式：{{value}}",
"slash.output.reasoning": "推理深度：{{value}}",
"slash.output.thinking": "思考：{{value}}",
"slash.output.streaming": "串流輸出：{{value}}",
"slash.output.longcontext": "長上下文：{{value}}",
"slash.error.title": "指令執行失敗",
"slash.error.badArgument": "/{{command}} 需要下列其中一個值：{{allowed}}"
```

`src/i18n/locales/ja.json`：

```json
"slash.output.applied": "適用しました",
"slash.output.mode": "実行モード: {{value}}",
"slash.output.reasoning": "推論の深さ: {{value}}",
"slash.output.thinking": "思考: {{value}}",
"slash.output.streaming": "ストリーミング: {{value}}",
"slash.output.longcontext": "ロングコンテキスト: {{value}}",
"slash.error.title": "コマンドが失敗しました",
"slash.error.badArgument": "/{{command}} には次のいずれかを指定してください: {{allowed}}"
```

`src/i18n/locales/ko.json`：

```json
"slash.output.applied": "적용됨",
"slash.output.mode": "실행 모드: {{value}}",
"slash.output.reasoning": "추론 깊이: {{value}}",
"slash.output.thinking": "사고: {{value}}",
"slash.output.streaming": "스트리밍: {{value}}",
"slash.output.longcontext": "긴 컨텍스트: {{value}}",
"slash.error.title": "명령 실행 실패",
"slash.error.badArgument": "/{{command}} 에는 다음 중 하나가 필요합니다: {{allowed}}"
```

- [ ] **Step 7: Add the locale parity test**

创建 `src/i18n/slash-command-locales.test.ts`（写法照搬既有的 `builtin-tool-locales.test.ts`）：

```ts
import { describe, expect, it } from "vitest";
import en from "./locales/en.json";
import ja from "./locales/ja.json";
import ko from "./locales/ko.json";
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";

const locales = { en, ja, ko, zhCN, zhTW };

describe("slash command localization", () => {
  it("keeps every English slash key present and non-empty in all supported locales", () => {
    const keys = Object.keys(en).filter((key) => key.startsWith("slash."));
    expect(keys.length).toBeGreaterThan(0);

    for (const [locale, messages] of Object.entries(locales)) {
      for (const key of keys) {
        expect(messages, `${locale} is missing ${key}`).toHaveProperty(key);
        expect(String(messages[key as keyof typeof messages]).trim(), `${locale}:${key}`).not.toBe("");
      }
    }
  });
});
```

- [ ] **Step 8: Run the locale test**

Run: `npx vitest run src/i18n/slash-command-locales.test.ts`
Expected: PASS — 1 test

- [ ] **Step 9: Commit**

```bash
git add src/services/slash-commands/ src/i18n/
git commit -m "feat: add runtime slash commands"
```

---

### Task 5: 会话与信息命令

**Files:**
- Create: `src/services/slash-commands/session-commands.ts`
- Test: `src/services/slash-commands/session-commands.test.ts`
- Modify: `src/services/slash-commands/command-catalog.ts`
- Modify: 五个 locale 文件

**Interfaces:**
- Consumes: Task 3 的类型、Task 2 的 `isOnePieceSession`
- Produces: `SESSION_COMMANDS: SlashCommand[]`（`/export` `/stop` `/status` `/usage`）

`/clear` 与 `/compact` 属于第二阶段，本任务不实现。

- [ ] **Step 1: Write the failing test**

创建 `src/services/slash-commands/session-commands.test.ts`：

```ts
import { describe, expect, it, vi } from "vitest";
import type { Session } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import { SESSION_COMMANDS } from "./session-commands";
import type { CommandContext, SlashCommand } from "./types";

const session = (agentId = "onepiece"): Session =>
  ({ id: "session-1", title: "S", agentId, interactionMode: "api" } as Session);

function context(overrides: { config?: Partial<ChatConfig>; isStreaming?: boolean } = {}) {
  const actions = {
    exportSession: vi.fn(), stop: vi.fn(),
    loadUsageSummary: vi.fn().mockResolvedValue({
      totalTokens: 1234, inputTokens: 1000, outputTokens: 234, responseCount: 7,
    }),
  };
  const ctx = {
    session: session(),
    config: {
      agentId: "onepiece", interactionMode: "api", executionMode: "plan",
      streaming: true, thinking: false, longContext: false,
      reasoningDepth: "medium", ...overrides.config,
    } as ChatConfig,
    isStreaming: overrides.isStreaming ?? false,
    chat: {
      setSessionExecutionMode: vi.fn(), setReasoningDepth: vi.fn(),
      setStreaming: vi.fn(), setThinking: vi.fn(), setLongContext: vi.fn(),
    },
    actions,
    navigate: { openAssociatedPlan: null, openDestination: vi.fn(), openSessionTab: vi.fn() },
    listAvailableCommands: () => [],
  } as unknown as CommandContext;
  return { ctx, actions };
}

const byName = (name: string): SlashCommand => {
  const command = SESSION_COMMANDS.find((entry) => entry.name === name);
  if (!command) throw new Error(`missing command: ${name}`);
  return command;
};

describe("session commands", () => {
  it("defers /clear and /compact to phase two", () => {
    const names = SESSION_COMMANDS.map((command) => command.name);
    expect(names).not.toContain("clear");
    expect(names).not.toContain("compact");
  });

  it("/export defaults to markdown", async () => {
    const { ctx, actions } = context();
    await byName("export").run(ctx, []);
    expect(actions.exportSession).toHaveBeenCalledWith(ctx.session, "markdown");
  });

  it("/export accepts json and the md alias", async () => {
    const json = context();
    await byName("export").run(json.ctx, ["json"]);
    expect(json.actions.exportSession).toHaveBeenCalledWith(json.ctx.session, "json");

    const md = context();
    await byName("export").run(md.ctx, ["md"]);
    expect(md.actions.exportSession).toHaveBeenCalledWith(md.ctx.session, "markdown");
  });

  it("/export rejects an unknown format", async () => {
    const { ctx, actions } = context();
    const outcome = await byName("export").run(ctx, ["pdf"]);
    expect(actions.exportSession).not.toHaveBeenCalled();
    expect(outcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.error.title", tone: "error",
        messages: [{ key: "slash.error.badArgument", params: { command: "export", allowed: "md, markdown, json" } }] },
    });
  });

  it("/stop only acts while streaming", async () => {
    const idle = context({ isStreaming: false });
    const outcome = await byName("stop").run(idle.ctx, []);
    expect(idle.actions.stop).not.toHaveBeenCalled();
    expect(outcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.error.title", tone: "error",
        messages: [{ key: "slash.error.notStreaming" }] },
    });

    const busy = context({ isStreaming: true });
    await byName("stop").run(busy.ctx, []);
    expect(busy.actions.stop).toHaveBeenCalled();
  });

  it("/status reports the current runtime switches", async () => {
    const { ctx } = context();
    const outcome = await byName("status").run(ctx, []);
    expect(outcome).toEqual({
      kind: "output",
      output: {
        titleKey: "slash.output.statusTitle", tone: "info",
        messages: [
          { key: "slash.output.mode", params: { value: "plan" } },
          { key: "slash.output.reasoning", params: { value: "medium" } },
          { key: "slash.output.thinking", params: { value: "off" } },
          { key: "slash.output.streaming", params: { value: "on" } },
          { key: "slash.output.longcontext", params: { value: "off" } },
        ],
      },
    });
  });

  it("/usage reports token totals from the service", async () => {
    const { ctx, actions } = context();
    const outcome = await byName("usage").run(ctx, []);
    expect(actions.loadUsageSummary).toHaveBeenCalledWith("session-1");
    expect(outcome).toEqual({
      kind: "output",
      output: {
        titleKey: "slash.output.usageTitle", tone: "info",
        messages: [
          { key: "slash.output.usageTotal", params: { value: 1234 } },
          { key: "slash.output.usageInput", params: { value: 1000 } },
          { key: "slash.output.usageOutput", params: { value: 234 } },
          { key: "slash.output.usageResponses", params: { value: 7 } },
        ],
      },
    });
  });

  it("/usage surfaces a service failure instead of throwing", async () => {
    const { ctx, actions } = context();
    actions.loadUsageSummary.mockRejectedValue(new Error("backend down"));
    const outcome = await byName("usage").run(ctx, []);
    expect(outcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.error.title", tone: "error",
        messages: [{ key: "slash.error.usageUnavailable" }] },
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/services/slash-commands/session-commands.test.ts`
Expected: FAIL — `Failed to resolve import "./session-commands"`

- [ ] **Step 3: Write the implementation**

创建 `src/services/slash-commands/session-commands.ts`：

```ts
import { isOnePieceSession } from "./command-availability";
import type { SessionExportFormat } from "../../types/agent";
import type { CommandOutcome, SlashCommand } from "./types";

const EXPORT_FORMATS: Record<string, SessionExportFormat> = {
  md: "markdown", markdown: "markdown", json: "json",
};

function error(key: string, params?: Record<string, string | number>): CommandOutcome {
  return { kind: "output", output: { titleKey: "slash.error.title", tone: "error", messages: [{ key, params }] } };
}

const onOff = (value: boolean): string => (value ? "on" : "off");

export const SESSION_COMMANDS: SlashCommand[] = [
  {
    name: "export", category: "session", argumentHint: "[md|json]", appliesTo: isOnePieceSession,
    run: async (context, args) => {
      const requested = args[0] ?? "md";
      const format = EXPORT_FORMATS[requested];
      if (!format) {
        return error("slash.error.badArgument", { command: "export", allowed: Object.keys(EXPORT_FORMATS).join(", ") });
      }
      context.actions.exportSession(context.session, format);
      return { kind: "output", output: { titleKey: "slash.output.applied", tone: "info", messages: [{ key: "slash.output.export", params: { value: format } }] } };
    },
  },
  {
    name: "stop", category: "session", appliesTo: isOnePieceSession,
    run: async (context) => {
      if (!context.isStreaming) return error("slash.error.notStreaming");
      context.actions.stop();
      return { kind: "handled" };
    },
  },
  {
    name: "status", category: "info", appliesTo: isOnePieceSession,
    run: async (context) => ({
      kind: "output",
      output: {
        titleKey: "slash.output.statusTitle", tone: "info",
        messages: [
          { key: "slash.output.mode", params: { value: context.config.executionMode } },
          { key: "slash.output.reasoning", params: { value: context.config.reasoningDepth ?? "low" } },
          { key: "slash.output.thinking", params: { value: onOff(context.config.thinking) } },
          { key: "slash.output.streaming", params: { value: onOff(context.config.streaming) } },
          { key: "slash.output.longcontext", params: { value: onOff(context.config.longContext) } },
        ],
      },
    }),
  },
  {
    name: "usage", category: "info", appliesTo: isOnePieceSession,
    run: async (context) => {
      try {
        const summary = await context.actions.loadUsageSummary(context.session.id);
        return {
          kind: "output",
          output: {
            titleKey: "slash.output.usageTitle", tone: "info",
            messages: [
              { key: "slash.output.usageTotal", params: { value: summary.totalTokens } },
              { key: "slash.output.usageInput", params: { value: summary.inputTokens } },
              { key: "slash.output.usageOutput", params: { value: summary.outputTokens } },
              { key: "slash.output.usageResponses", params: { value: summary.responseCount } },
            ],
          },
        };
      } catch {
        // The panel is the only feedback channel a command has, so a failed lookup has to be
        // reported here rather than thrown into a boundary the user never sees.
        return error("slash.error.usageUnavailable");
      }
    },
  },
];
```

- [ ] **Step 4: Extend the catalog**

修改 `src/services/slash-commands/command-catalog.ts`，整个文件替换为：

```ts
import { RUNTIME_COMMANDS } from "./runtime-commands";
import { SESSION_COMMANDS } from "./session-commands";
import type { SlashCommand } from "./types";

export const SLASH_COMMANDS: SlashCommand[] = [...RUNTIME_COMMANDS, ...SESSION_COMMANDS];
```

- [ ] **Step 5: Run test to verify it passes**

Run: `npx vitest run src/services/slash-commands/session-commands.test.ts`
Expected: PASS — 8 tests

- [ ] **Step 6: Add the i18n keys**

追加到五个 locale 文件。

`en.json`：

```json
"slash.output.export": "Export started: {{value}}",
"slash.output.statusTitle": "Session status",
"slash.output.usageTitle": "Token usage",
"slash.output.usageTotal": "Total tokens: {{value}}",
"slash.output.usageInput": "Input tokens: {{value}}",
"slash.output.usageOutput": "Output tokens: {{value}}",
"slash.output.usageResponses": "Responses: {{value}}",
"slash.error.notStreaming": "Nothing is generating right now.",
"slash.error.usageUnavailable": "Token usage could not be read."
```

`zh-CN.json`：

```json
"slash.output.export": "已开始导出：{{value}}",
"slash.output.statusTitle": "会话状态",
"slash.output.usageTitle": "Token 用量",
"slash.output.usageTotal": "总 Token：{{value}}",
"slash.output.usageInput": "输入 Token：{{value}}",
"slash.output.usageOutput": "输出 Token：{{value}}",
"slash.output.usageResponses": "回复次数：{{value}}",
"slash.error.notStreaming": "当前没有正在生成的内容。",
"slash.error.usageUnavailable": "无法读取 Token 用量。"
```

`zh-TW.json`：

```json
"slash.output.export": "已開始匯出：{{value}}",
"slash.output.statusTitle": "工作階段狀態",
"slash.output.usageTitle": "Token 用量",
"slash.output.usageTotal": "總 Token：{{value}}",
"slash.output.usageInput": "輸入 Token：{{value}}",
"slash.output.usageOutput": "輸出 Token：{{value}}",
"slash.output.usageResponses": "回覆次數：{{value}}",
"slash.error.notStreaming": "目前沒有正在生成的內容。",
"slash.error.usageUnavailable": "無法讀取 Token 用量。"
```

`ja.json`：

```json
"slash.output.export": "エクスポートを開始しました: {{value}}",
"slash.output.statusTitle": "セッションの状態",
"slash.output.usageTitle": "トークン使用量",
"slash.output.usageTotal": "合計トークン: {{value}}",
"slash.output.usageInput": "入力トークン: {{value}}",
"slash.output.usageOutput": "出力トークン: {{value}}",
"slash.output.usageResponses": "応答回数: {{value}}",
"slash.error.notStreaming": "現在生成中の処理はありません。",
"slash.error.usageUnavailable": "トークン使用量を取得できませんでした。"
```

`ko.json`：

```json
"slash.output.export": "내보내기를 시작했습니다: {{value}}",
"slash.output.statusTitle": "세션 상태",
"slash.output.usageTitle": "토큰 사용량",
"slash.output.usageTotal": "총 토큰: {{value}}",
"slash.output.usageInput": "입력 토큰: {{value}}",
"slash.output.usageOutput": "출력 토큰: {{value}}",
"slash.output.usageResponses": "응답 횟수: {{value}}",
"slash.error.notStreaming": "현재 생성 중인 작업이 없습니다.",
"slash.error.usageUnavailable": "토큰 사용량을 읽을 수 없습니다."
```

- [ ] **Step 7: Run the locale parity test**

Run: `npx vitest run src/i18n/slash-command-locales.test.ts`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add src/services/slash-commands/ src/i18n/
git commit -m "feat: add session and info slash commands"
```

---

### Task 6: 导航命令

**Files:**
- Create: `src/services/slash-commands/navigation-commands.ts`
- Test: `src/services/slash-commands/navigation-commands.test.ts`
- Modify: `src/services/slash-commands/command-catalog.ts`
- Modify: 五个 locale 文件

**Interfaces:**
- Consumes: `SlashCommandNavigation`、`SlashCommandDestination`、`SessionTabId`
- Produces: `NAVIGATION_COMMANDS: SlashCommand[]`

`/plan`（单数）打开当前会话关联的那次计划运行，仅当 `navigate.openAssociatedPlan` 非 null 时可用；`/plans`（复数）切到全局计划中心。两者在 `/help` 中必须都出现且描述不同。

- [ ] **Step 1: Write the failing test**

创建 `src/services/slash-commands/navigation-commands.test.ts`：

```ts
import { describe, expect, it, vi } from "vitest";
import type { Session } from "../../types/agent";
import { NAVIGATION_COMMANDS } from "./navigation-commands";
import type { CommandContext, SlashCommand } from "./types";

const session = (agentId = "onepiece"): Session =>
  ({ id: "s", title: "S", agentId, interactionMode: "api" } as Session);

function context(openAssociatedPlan: (() => void) | null = null) {
  const navigate = { openAssociatedPlan, openDestination: vi.fn(), openSessionTab: vi.fn() };
  const ctx = {
    session: session(), config: {}, isStreaming: false,
    chat: {}, actions: {}, navigate, listAvailableCommands: () => [],
  } as unknown as CommandContext;
  return { ctx, navigate };
}

const byName = (name: string): SlashCommand => {
  const command = NAVIGATION_COMMANDS.find((entry) => entry.name === name);
  if (!command) throw new Error(`missing command: ${name}`);
  return command;
};

describe("navigation commands", () => {
  it("/todo, /plans and /loops switch destination", async () => {
    for (const [name, destination] of [["todo", "todo-board"], ["plans", "plans"], ["loops", "loops"]] as const) {
      const { ctx, navigate } = context();
      await byName(name).run(ctx, []);
      expect(navigate.openDestination).toHaveBeenCalledWith(destination);
    }
  });

  it("exposes one command per workspace tab except chat", async () => {
    for (const tab of ["logs", "files", "changes", "documents", "terminal", "shell", "traces", "report"] as const) {
      const { ctx, navigate } = context();
      await byName(tab).run(ctx, []);
      expect(navigate.openSessionTab).toHaveBeenCalledWith(tab);
    }
  });

  it("/plan is unavailable without an associated plan run", () => {
    expect(byName("plan").appliesTo(session(), { hasAssociatedPlan: false })).toBe(false);
    expect(byName("plan").appliesTo(session(), { hasAssociatedPlan: true })).toBe(true);
    expect(byName("plan").appliesTo(session("claude-code"), { hasAssociatedPlan: true })).toBe(false);
  });

  it("/plan opens the associated run when one exists", async () => {
    const open = vi.fn();
    const { ctx } = context(open);
    await byName("plan").run(ctx, []);
    expect(open).toHaveBeenCalled();
  });

  it("/plan and /plans are distinct commands", () => {
    expect(byName("plan").name).not.toBe(byName("plans").name);
    expect(byName("plan").category).toBe("navigation");
    expect(byName("plans").category).toBe("navigation");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/services/slash-commands/navigation-commands.test.ts`
Expected: FAIL — `Failed to resolve import "./navigation-commands"`

- [ ] **Step 3: Write the implementation**

创建 `src/services/slash-commands/navigation-commands.ts`：

```ts
import { isOnePieceSession } from "./command-availability";
import type { SessionTabId } from "../../session-workspace/session-tab-bar";
import type { SlashCommand, SlashCommandDestination } from "./types";

const TAB_COMMANDS: SessionTabId[] = [
  "logs", "files", "changes", "documents", "terminal", "shell", "traces", "report",
];

const DESTINATION_COMMANDS: Array<{ name: string; destination: SlashCommandDestination }> = [
  { name: "todo", destination: "todo-board" },
  { name: "plans", destination: "plans" },
  { name: "loops", destination: "loops" },
];

export const NAVIGATION_COMMANDS: SlashCommand[] = [
  ...DESTINATION_COMMANDS.map(({ name, destination }): SlashCommand => ({
    name, category: "navigation", appliesTo: isOnePieceSession,
    run: async (context) => {
      context.navigate.openDestination(destination);
      return { kind: "handled" };
    },
  })),
  ...TAB_COMMANDS.map((tab): SlashCommand => ({
    name: tab, category: "navigation", appliesTo: isOnePieceSession,
    run: async (context) => {
      context.navigate.openSessionTab(tab);
      return { kind: "handled" };
    },
  })),
  {
    name: "plan", category: "navigation",
    // Availability comes in as an argument rather than module state so the predicate stays pure
    // and the test suite cannot leak one case's setup into the next.
    appliesTo: (session, capabilities) => isOnePieceSession(session) && capabilities.hasAssociatedPlan,
    run: async (context) => {
      context.navigate.openAssociatedPlan?.();
      return { kind: "handled" };
    },
  },
];
```

- [ ] **Step 4: Extend the catalog**

修改 `src/services/slash-commands/command-catalog.ts`，整个文件替换为：

```ts
import { NAVIGATION_COMMANDS } from "./navigation-commands";
import { RUNTIME_COMMANDS } from "./runtime-commands";
import { SESSION_COMMANDS } from "./session-commands";
import type { SlashCommand } from "./types";

export const SLASH_COMMANDS: SlashCommand[] = [
  ...RUNTIME_COMMANDS, ...SESSION_COMMANDS, ...NAVIGATION_COMMANDS,
];
```

- [ ] **Step 5: Run test to verify it passes**

Run: `npx vitest run src/services/slash-commands/navigation-commands.test.ts`
Expected: PASS — 5 tests

- [ ] **Step 6: Add the i18n keys**

`/help` 需要每条命令的描述。追加到五个 locale。

`en.json`：

```json
"slash.command.todo.description": "Open the todo board",
"slash.command.plans.description": "Open the plan center",
"slash.command.loops.description": "Open the loop center",
"slash.command.plan.description": "Open this session's associated plan run",
"slash.command.logs.description": "Open the logs tab",
"slash.command.files.description": "Open the files tab",
"slash.command.changes.description": "Open the changes tab",
"slash.command.documents.description": "Open the documents tab",
"slash.command.terminal.description": "Open the terminal tab",
"slash.command.shell.description": "Open the shell tab",
"slash.command.traces.description": "Open the traces tab",
"slash.command.report.description": "Open the report tab"
```

`zh-CN.json`：

```json
"slash.command.todo.description": "打开待办看板",
"slash.command.plans.description": "打开计划中心",
"slash.command.loops.description": "打开循环中心",
"slash.command.plan.description": "打开本会话关联的计划运行",
"slash.command.logs.description": "打开日志页签",
"slash.command.files.description": "打开文件页签",
"slash.command.changes.description": "打开变更页签",
"slash.command.documents.description": "打开文档页签",
"slash.command.terminal.description": "打开终端页签",
"slash.command.shell.description": "打开 Shell 页签",
"slash.command.traces.description": "打开追踪页签",
"slash.command.report.description": "打开报告页签"
```

`zh-TW.json`：

```json
"slash.command.todo.description": "開啟待辦看板",
"slash.command.plans.description": "開啟計畫中心",
"slash.command.loops.description": "開啟迴圈中心",
"slash.command.plan.description": "開啟本工作階段關聯的計畫執行",
"slash.command.logs.description": "開啟記錄頁籤",
"slash.command.files.description": "開啟檔案頁籤",
"slash.command.changes.description": "開啟變更頁籤",
"slash.command.documents.description": "開啟文件頁籤",
"slash.command.terminal.description": "開啟終端機頁籤",
"slash.command.shell.description": "開啟 Shell 頁籤",
"slash.command.traces.description": "開啟追蹤頁籤",
"slash.command.report.description": "開啟報告頁籤"
```

`ja.json`：

```json
"slash.command.todo.description": "タスクボードを開く",
"slash.command.plans.description": "プランセンターを開く",
"slash.command.loops.description": "ループセンターを開く",
"slash.command.plan.description": "このセッションに紐づくプラン実行を開く",
"slash.command.logs.description": "ログタブを開く",
"slash.command.files.description": "ファイルタブを開く",
"slash.command.changes.description": "変更タブを開く",
"slash.command.documents.description": "ドキュメントタブを開く",
"slash.command.terminal.description": "ターミナルタブを開く",
"slash.command.shell.description": "シェルタブを開く",
"slash.command.traces.description": "トレースタブを開く",
"slash.command.report.description": "レポートタブを開く"
```

`ko.json`：

```json
"slash.command.todo.description": "할 일 보드 열기",
"slash.command.plans.description": "플랜 센터 열기",
"slash.command.loops.description": "루프 센터 열기",
"slash.command.plan.description": "이 세션에 연결된 플랜 실행 열기",
"slash.command.logs.description": "로그 탭 열기",
"slash.command.files.description": "파일 탭 열기",
"slash.command.changes.description": "변경 탭 열기",
"slash.command.documents.description": "문서 탭 열기",
"slash.command.terminal.description": "터미널 탭 열기",
"slash.command.shell.description": "셸 탭 열기",
"slash.command.traces.description": "트레이스 탭 열기",
"slash.command.report.description": "리포트 탭 열기"
```

- [ ] **Step 7: Run the locale parity test**

Run: `npx vitest run src/i18n/slash-command-locales.test.ts`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add src/services/slash-commands/ src/i18n/
git commit -m "feat: add navigation slash commands"
```

---

### Task 7: `/help` 命令

**Files:**
- Create: `src/services/slash-commands/help-command.ts`
- Test: `src/services/slash-commands/help-command.test.ts`
- Modify: `src/services/slash-commands/command-catalog.ts`
- Modify: 五个 locale 文件

**Interfaces:**
- Consumes: `CommandContext.listAvailableCommands`
- Produces: `HELP_COMMAND: SlashCommand`

- [ ] **Step 1: Write the failing test**

创建 `src/services/slash-commands/help-command.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import type { Session } from "../../types/agent";
import { HELP_COMMAND } from "./help-command";
import type { CommandContext, SlashCommand } from "./types";

const session = (agentId = "onepiece"): Session =>
  ({ id: "s", title: "S", agentId, interactionMode: "api" } as Session);

const stub = (name: string, argumentHint?: string): SlashCommand => ({
  name, category: "runtime", argumentHint, appliesTo: () => true,
  run: async () => ({ kind: "handled" }),
});

describe("/help", () => {
  it("lists the commands the dispatcher says are available", async () => {
    const context = {
      session: session(),
      listAvailableCommands: () => [stub("mode", "<inherit|plan|execute>"), stub("status")],
    } as unknown as CommandContext;

    const outcome = await HELP_COMMAND.run(context, []);
    expect(outcome).toEqual({
      kind: "output",
      output: {
        titleKey: "slash.output.helpTitle", tone: "info",
        messages: [
          { key: "slash.output.helpEntry", params: { invocation: "/mode <inherit|plan|execute>", description: "slash.command.mode.description" } },
          { key: "slash.output.helpEntry", params: { invocation: "/status", description: "slash.command.status.description" } },
        ],
      },
    });
  });

  it("is available in any OnePiece session", () => {
    const capabilities = { hasAssociatedPlan: false };
    expect(HELP_COMMAND.appliesTo(session("onepiece"), capabilities)).toBe(true);
    expect(HELP_COMMAND.appliesTo(session("claude-code"), capabilities)).toBe(false);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/services/slash-commands/help-command.test.ts`
Expected: FAIL — `Failed to resolve import "./help-command"`

- [ ] **Step 3: Write the implementation**

创建 `src/services/slash-commands/help-command.ts`：

```ts
import { isOnePieceSession } from "./command-availability";
import type { SlashCommand } from "./types";

function invocation(command: SlashCommand): string {
  return command.argumentHint ? `/${command.name} ${command.argumentHint}` : `/${command.name}`;
}

export const HELP_COMMAND: SlashCommand = {
  name: "help", aliases: ["?"], category: "info", appliesTo: isOnePieceSession,
  run: async (context) => ({
    kind: "output",
    output: {
      titleKey: "slash.output.helpTitle", tone: "info",
      // The description is passed through as a key so the output panel translates it in one place,
      // the same way every other command message is handled.
      messages: context.listAvailableCommands().map((command) => ({
        key: "slash.output.helpEntry",
        params: { invocation: invocation(command), description: `slash.command.${command.name}.description` },
      })),
    },
  }),
};
```

- [ ] **Step 4: Extend the catalog**

修改 `src/services/slash-commands/command-catalog.ts`，整个文件替换为：

```ts
import { HELP_COMMAND } from "./help-command";
import { NAVIGATION_COMMANDS } from "./navigation-commands";
import { RUNTIME_COMMANDS } from "./runtime-commands";
import { SESSION_COMMANDS } from "./session-commands";
import type { SlashCommand } from "./types";

export const SLASH_COMMANDS: SlashCommand[] = [
  ...RUNTIME_COMMANDS, ...SESSION_COMMANDS, ...NAVIGATION_COMMANDS, HELP_COMMAND,
];
```

- [ ] **Step 5: Run test to verify it passes**

Run: `npx vitest run src/services/slash-commands/help-command.test.ts`
Expected: PASS — 2 tests

- [ ] **Step 6: Add the i18n keys**

补齐 `/help` 自身的文案，以及 Task 4/5 命令缺失的描述键。

`en.json`：

```json
"slash.output.helpTitle": "Available commands",
"slash.output.helpEntry": "{{invocation}} — {{description}}",
"slash.command.help.description": "List available commands",
"slash.command.mode.description": "Set the execution mode",
"slash.command.reasoning.description": "Set the reasoning depth",
"slash.command.thinking.description": "Toggle thinking",
"slash.command.streaming.description": "Toggle streaming",
"slash.command.longcontext.description": "Toggle long context",
"slash.command.export.description": "Export this session",
"slash.command.stop.description": "Stop the current generation",
"slash.command.status.description": "Show the current runtime switches",
"slash.command.usage.description": "Show token usage for this session"
```

`zh-CN.json`：

```json
"slash.output.helpTitle": "可用命令",
"slash.output.helpEntry": "{{invocation}} — {{description}}",
"slash.command.help.description": "列出可用命令",
"slash.command.mode.description": "设置执行模式",
"slash.command.reasoning.description": "设置推理深度",
"slash.command.thinking.description": "切换思考开关",
"slash.command.streaming.description": "切换流式输出",
"slash.command.longcontext.description": "切换长上下文",
"slash.command.export.description": "导出当前会话",
"slash.command.stop.description": "停止当前生成",
"slash.command.status.description": "显示当前运行时开关",
"slash.command.usage.description": "显示本会话的 Token 用量"
```

`zh-TW.json`：

```json
"slash.output.helpTitle": "可用指令",
"slash.output.helpEntry": "{{invocation}} — {{description}}",
"slash.command.help.description": "列出可用指令",
"slash.command.mode.description": "設定執行模式",
"slash.command.reasoning.description": "設定推理深度",
"slash.command.thinking.description": "切換思考開關",
"slash.command.streaming.description": "切換串流輸出",
"slash.command.longcontext.description": "切換長上下文",
"slash.command.export.description": "匯出目前工作階段",
"slash.command.stop.description": "停止目前生成",
"slash.command.status.description": "顯示目前執行時開關",
"slash.command.usage.description": "顯示本工作階段的 Token 用量"
```

`ja.json`：

```json
"slash.output.helpTitle": "利用可能なコマンド",
"slash.output.helpEntry": "{{invocation}} — {{description}}",
"slash.command.help.description": "利用可能なコマンドを一覧表示",
"slash.command.mode.description": "実行モードを設定",
"slash.command.reasoning.description": "推論の深さを設定",
"slash.command.thinking.description": "思考の有効・無効を切り替え",
"slash.command.streaming.description": "ストリーミングを切り替え",
"slash.command.longcontext.description": "ロングコンテキストを切り替え",
"slash.command.export.description": "このセッションをエクスポート",
"slash.command.stop.description": "現在の生成を停止",
"slash.command.status.description": "現在の実行時スイッチを表示",
"slash.command.usage.description": "このセッションのトークン使用量を表示"
```

`ko.json`：

```json
"slash.output.helpTitle": "사용 가능한 명령",
"slash.output.helpEntry": "{{invocation}} — {{description}}",
"slash.command.help.description": "사용 가능한 명령 목록 표시",
"slash.command.mode.description": "실행 모드 설정",
"slash.command.reasoning.description": "추론 깊이 설정",
"slash.command.thinking.description": "사고 켜기/끄기",
"slash.command.streaming.description": "스트리밍 켜기/끄기",
"slash.command.longcontext.description": "긴 컨텍스트 켜기/끄기",
"slash.command.export.description": "이 세션 내보내기",
"slash.command.stop.description": "현재 생성 중지",
"slash.command.status.description": "현재 런타임 스위치 표시",
"slash.command.usage.description": "이 세션의 토큰 사용량 표시"
```

- [ ] **Step 7: Run the locale parity test**

Run: `npx vitest run src/i18n/slash-command-locales.test.ts`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add src/services/slash-commands/ src/i18n/
git commit -m "feat: add slash help command"
```

---

### Task 8: 输出面板组件

**Files:**
- Create: `src/components/chat/SlashCommandOutput.tsx`
- Test: `src/components/chat/SlashCommandOutput.test.tsx`
- Modify: 五个 locale 文件

**Interfaces:**
- Consumes: `CommandOutput`、`CommandMessage`（Task 3）
- Produces: `SlashCommandOutput({ output, onDismiss }: { output: CommandOutput | null; onDismiss: () => void })`

`/help` 的条目把描述作为 key 放在 `params.description` 里，所以本组件在插值前必须先翻译该参数。

- [ ] **Step 1: Write the failing test**

创建 `src/components/chat/SlashCommandOutput.test.tsx`：

```tsx
import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithAppProviders } from "../../test/render";
import { SlashCommandOutput } from "./SlashCommandOutput";

describe("SlashCommandOutput", () => {
  it("renders nothing when there is no output", () => {
    const { container } = renderWithAppProviders(<SlashCommandOutput output={null} onDismiss={() => undefined} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("translates the title and each message", () => {
    renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={() => undefined}
        output={{ titleKey: "slash.output.applied", tone: "info", messages: [{ key: "slash.output.mode", params: { value: "plan" } }] }}
      />,
    );
    expect(screen.getByTestId("slash-command-output")).toBeInTheDocument();
    expect(screen.getByText("Applied")).toBeInTheDocument();
    expect(screen.getByText("Execution mode: plan")).toBeInTheDocument();
  });

  it("translates a help entry's description parameter before interpolating", () => {
    renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={() => undefined}
        output={{
          titleKey: "slash.output.helpTitle", tone: "info",
          messages: [{ key: "slash.output.helpEntry", params: { invocation: "/status", description: "slash.command.status.description" } }],
        }}
      />,
    );
    expect(screen.getByText("/status — Show the current runtime switches")).toBeInTheDocument();
  });

  it("marks an error tone for assistive technology", () => {
    renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={() => undefined}
        output={{ titleKey: "slash.error.title", tone: "error", messages: [{ key: "slash.error.notStreaming" }] }}
      />,
    );
    expect(screen.getByTestId("slash-command-output")).toHaveAttribute("data-tone", "error");
  });

  it("dismisses on the close button", async () => {
    const onDismiss = vi.fn();
    const { user } = renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={onDismiss}
        output={{ titleKey: "slash.output.applied", tone: "info", messages: [] }}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Dismiss command output" }));
    expect(onDismiss).toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/components/chat/SlashCommandOutput.test.tsx`
Expected: FAIL — `Failed to resolve import "./SlashCommandOutput"`

- [ ] **Step 3: Write the implementation**

创建 `src/components/chat/SlashCommandOutput.tsx`：

```tsx
import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { CommandMessage, CommandOutput } from "../../services/slash-commands/types";

/**
 * Command output lives outside the message list on purpose: `invalidateRuntime` refetches
 * `["messages", sessionId]` after every send, which would wipe anything injected locally.
 */
export function SlashCommandOutput({
  onDismiss,
  output,
}: {
  onDismiss: () => void;
  output: CommandOutput | null;
}) {
  const { t } = useTranslation();
  if (!output) return null;

  function render(message: CommandMessage): string {
    const params = message.params ?? {};
    // `/help` ships its per-command description as a key, so it has to be resolved before the
    // outer string interpolates it.
    const resolved = typeof params.description === "string"
      ? { ...params, description: t(params.description) }
      : params;
    return t(message.key, resolved);
  }

  return (
    <div
      className="ucd-panel absolute bottom-full left-0 z-20 mb-2 grid max-h-56 w-full gap-1 overflow-y-auto rounded-md p-2 text-xs shadow-lg"
      data-testid="slash-command-output"
      data-tone={output.tone}
      role={output.tone === "error" ? "alert" : "status"}
    >
      <div className="flex items-center gap-2">
        <span className="min-w-0 flex-1 truncate font-semibold">{t(output.titleKey)}</span>
        <button
          aria-label={t("slash.output.dismiss")}
          className="rounded text-muted-foreground hover:text-foreground"
          onClick={onDismiss}
          type="button"
        >
          <X aria-hidden="true" className="h-3.5 w-3.5" />
        </button>
      </div>
      {output.messages.map((message, index) => (
        <p className="text-muted-foreground" key={`${message.key}-${index}`}>{render(message)}</p>
      ))}
    </div>
  );
}
```

- [ ] **Step 4: Add the i18n key**

五个 locale 各加一行：

- `en.json`: `"slash.output.dismiss": "Dismiss command output"`
- `zh-CN.json`: `"slash.output.dismiss": "关闭命令输出"`
- `zh-TW.json`: `"slash.output.dismiss": "關閉指令輸出"`
- `ja.json`: `"slash.output.dismiss": "コマンド出力を閉じる"`
- `ko.json`: `"slash.output.dismiss": "명령 출력 닫기"`

- [ ] **Step 5: Run test to verify it passes**

Run: `npx vitest run src/components/chat/SlashCommandOutput.test.tsx src/i18n/slash-command-locales.test.ts`
Expected: PASS — 5 + 1 tests

- [ ] **Step 6: Commit**

```bash
git add src/components/chat/SlashCommandOutput.tsx src/components/chat/SlashCommandOutput.test.tsx src/i18n/
git commit -m "feat: add slash command output panel"
```

---

### Task 9: 补全下拉组件

**Files:**
- Create: `src/components/chat/SlashCommandCompletion.tsx`
- Test: `src/components/chat/SlashCommandCompletion.test.tsx`
- Modify: 五个 locale 文件

**Interfaces:**
- Consumes: `SlashCommand`（Task 3）
- Produces: `SlashCommandCompletion({ onSelect, options }: { onSelect: (name: string) => void; options: SlashCommand[] })`

结构照搬 `SeatMentionCompletion.tsx`（同目录，47 行）：options 为空时返回 null，外层带 `role="group"` 与 `aria-label`。

- [ ] **Step 1: Write the failing test**

创建 `src/components/chat/SlashCommandCompletion.test.tsx`：

```tsx
import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithAppProviders } from "../../test/render";
import type { SlashCommand } from "../../services/slash-commands/types";
import { SlashCommandCompletion } from "./SlashCommandCompletion";

const command = (name: string, argumentHint?: string): SlashCommand => ({
  name, category: "runtime", argumentHint, appliesTo: () => true,
  run: async () => ({ kind: "handled" }),
});

describe("SlashCommandCompletion", () => {
  it("renders nothing when there are no options", () => {
    const { container } = renderWithAppProviders(<SlashCommandCompletion onSelect={() => undefined} options={[]} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("shows the invocation and the translated description", () => {
    renderWithAppProviders(
      <SlashCommandCompletion onSelect={() => undefined} options={[command("mode", "<inherit|plan|execute>")]} />,
    );
    expect(screen.getByText("/mode <inherit|plan|execute>")).toBeInTheDocument();
    expect(screen.getByText("Set the execution mode")).toBeInTheDocument();
  });

  it("reports the selected command name", async () => {
    const onSelect = vi.fn();
    const { user } = renderWithAppProviders(
      <SlashCommandCompletion onSelect={onSelect} options={[command("status"), command("usage")]} />,
    );
    await user.click(screen.getByRole("button", { name: /\/usage/ }));
    expect(onSelect).toHaveBeenCalledWith("usage");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/components/chat/SlashCommandCompletion.test.tsx`
Expected: FAIL — `Failed to resolve import "./SlashCommandCompletion"`

- [ ] **Step 3: Write the implementation**

创建 `src/components/chat/SlashCommandCompletion.tsx`：

```tsx
import { useTranslation } from "react-i18next";
import type { SlashCommand } from "../../services/slash-commands/types";

export function SlashCommandCompletion({
  onSelect,
  options,
}: {
  onSelect: (name: string) => void;
  options: SlashCommand[];
}) {
  const { t } = useTranslation();
  if (options.length === 0) return null;

  return (
    <div aria-label={t("slash.completion.title")} className="grid gap-0.5 text-sm" role="group">
      <p className="px-2 py-1 text-[11px] font-semibold uppercase text-muted-foreground">{t("slash.completion.title")}</p>
      {options.map((option) => (
        <button
          className="ucd-interactive flex items-center gap-2 rounded px-2 py-1.5 text-left"
          key={option.name}
          onClick={() => onSelect(option.name)}
          type="button"
        >
          <span className="shrink-0 font-medium">
            {option.argumentHint ? `/${option.name} ${option.argumentHint}` : `/${option.name}`}
          </span>
          <span className="min-w-0 flex-1 truncate text-muted-foreground">
            {t(`slash.command.${option.name}.description`)}
          </span>
        </button>
      ))}
    </div>
  );
}
```

- [ ] **Step 4: Add the i18n key**

五个 locale 各加一行：

- `en.json`: `"slash.completion.title": "Commands"`
- `zh-CN.json`: `"slash.completion.title": "命令"`
- `zh-TW.json`: `"slash.completion.title": "指令"`
- `ja.json`: `"slash.completion.title": "コマンド"`
- `ko.json`: `"slash.completion.title": "명령"`

- [ ] **Step 5: Run test to verify it passes**

Run: `npx vitest run src/components/chat/SlashCommandCompletion.test.tsx src/i18n/slash-command-locales.test.ts`
Expected: PASS — 3 + 1 tests

- [ ] **Step 6: Commit**

```bash
git add src/components/chat/SlashCommandCompletion.tsx src/components/chat/SlashCommandCompletion.test.tsx src/i18n/
git commit -m "feat: add slash command completion dropdown"
```

---

### Task 10: 调度 hook

**Files:**
- Create: `src/services/slash-commands/use-slash-commands.ts`
- Test: `src/services/slash-commands/use-slash-commands.test.tsx`

**Interfaces:**
- Consumes: `parseCommandInput`（Task 1）、`slashCommandsEnabled`（Task 2）、`findCommand`/`listCommands`/`CommandCapabilities`（Task 3）、`SLASH_COMMANDS`（Task 7）
- Produces:

```ts
useSlashCommands(input: {
  session: Session | null;
  config: ChatConfig;
  isStreaming: boolean;
  actions: CommandContext["actions"];
  chat: CommandContext["chat"];
  navigate: SlashCommandNavigation;
  onError: (source: string, reason: unknown) => void;
}): {
  output: CommandOutput | null;
  dismissOutput: () => void;
  suggestions: SlashCommand[];
  suggestionQuery: string | null;
  dispatch: (draft: string) => DispatchResult;
  updateSuggestions: (draft: string) => void;
  completeDraft: (name: string) => string;
}

type DispatchResult = { kind: "message" } | { kind: "literal"; content: string } | { kind: "handled" };
```

两个入口职责严格分开，混用会导致每敲一个字符就执行一次命令：

- `updateSuggestions` 由输入框的 `onChange` 每次按键调用，**只**刷新补全列表，绝不执行任何命令
- `dispatch` 只在提交时调用，**同步**返回是否已接管输入（决定要不要放行给 `model.submit()`）；命令本身可以是异步的，输出随后设入 state

- [ ] **Step 1: Write the failing test**

创建 `src/services/slash-commands/use-slash-commands.test.tsx`：

```tsx
import { describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { Session } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import { useSlashCommands } from "./use-slash-commands";

const session = (agentId = "onepiece"): Session =>
  ({ id: "session-1", title: "S", agentId, interactionMode: "api" } as Session);

const config: ChatConfig = {
  agentId: "onepiece", interactionMode: "api", executionMode: "plan",
  streaming: true, thinking: false, longContext: false, reasoningDepth: "low",
};

function setup(overrides: {
  session?: Session | null;
  isStreaming?: boolean;
  openAssociatedPlan?: () => void;
} = {}) {
  const chat = {
    setSessionExecutionMode: vi.fn(), setReasoningDepth: vi.fn(),
    setStreaming: vi.fn(), setThinking: vi.fn(), setLongContext: vi.fn(),
  };
  const actions = {
    exportSession: vi.fn(), stop: vi.fn(),
    loadUsageSummary: vi.fn().mockResolvedValue({ totalTokens: 1, inputTokens: 1, outputTokens: 0, responseCount: 1 }),
  };
  const navigate = {
    openAssociatedPlan: overrides.openAssociatedPlan ?? null,
    openDestination: vi.fn(),
    openSessionTab: vi.fn(),
  };
  const onError = vi.fn();
  const rendered = renderHook(() => useSlashCommands({
    session: overrides.session === undefined ? session() : overrides.session,
    config, isStreaming: overrides.isStreaming ?? false, chat, actions, navigate, onError,
  }));
  return { ...rendered, chat, actions, navigate, onError };
}

describe("useSlashCommands", () => {
  it("passes ordinary prose through", () => {
    const { result } = setup();
    expect(result.current.dispatch("hello")).toEqual({ kind: "message" });
  });

  it("unescapes a doubled slash into literal content", () => {
    const { result } = setup();
    expect(result.current.dispatch("//help")).toEqual({ kind: "literal", content: "/help" });
  });

  it("passes everything through when the session is not eligible", () => {
    const { result } = setup({ session: session("claude-code") });
    expect(result.current.dispatch("/help")).toEqual({ kind: "message" });
  });

  it("passes everything through when there is no session", () => {
    const { result } = setup({ session: null });
    expect(result.current.dispatch("/help")).toEqual({ kind: "message" });
  });

  it("runs a known command and keeps it away from the model", async () => {
    const { result, chat } = setup();
    act(() => { expect(result.current.dispatch("/mode execute")).toEqual({ kind: "handled" }); });
    expect(chat.setSessionExecutionMode).toHaveBeenCalledWith("execute");
    await waitFor(() => expect(result.current.output?.titleKey).toBe("slash.output.applied"));
  });

  it("reports an unknown command without forwarding it", async () => {
    const { result } = setup();
    act(() => { expect(result.current.dispatch("/nope")).toEqual({ kind: "handled" }); });
    await waitFor(() => expect(result.current.output).toEqual({
      titleKey: "slash.error.title", tone: "error",
      messages: [{ key: "slash.error.unknown", params: { command: "nope" } }],
    }));
  });

  it("dismisses output on request", async () => {
    const { result } = setup();
    act(() => { result.current.dispatch("/status"); });
    await waitFor(() => expect(result.current.output).not.toBeNull());
    act(() => { result.current.dismissOutput(); });
    expect(result.current.output).toBeNull();
  });

  it("suggests commands while a bare slash prefix is being typed", () => {
    const { result } = setup();
    expect(result.current.suggestionQuery).toBeNull();

    act(() => { result.current.updateSuggestions("/mod"); });
    expect(result.current.suggestionQuery).toBe("mod");
    expect(result.current.suggestions.map((entry) => entry.name)).toEqual(["mode"]);

    act(() => { result.current.updateSuggestions("/mode plan"); });
    expect(result.current.suggestionQuery).toBeNull();
    expect(result.current.suggestions).toEqual([]);
  });

  it("never executes a command from updateSuggestions", () => {
    const { result, chat } = setup();
    act(() => { result.current.updateSuggestions("/mode execute"); });
    expect(chat.setSessionExecutionMode).not.toHaveBeenCalled();
    expect(result.current.output).toBeNull();
  });

  it("completes a draft into a ready-to-run invocation", () => {
    const { result } = setup();
    expect(result.current.completeDraft("mode")).toBe("/mode ");
  });

  it("offers /plan only when the session has an associated plan run", () => {
    const without = setup();
    act(() => { without.result.current.updateSuggestions("/pla"); });
    expect(without.result.current.suggestions.map((entry) => entry.name)).toEqual(["plans"]);

    const withPlan = setup({ openAssociatedPlan: () => undefined });
    act(() => { withPlan.result.current.updateSuggestions("/pla"); });
    expect(withPlan.result.current.suggestions.map((entry) => entry.name)).toEqual(["plan", "plans"]);
  });

  it("reports a handler that throws through onError", async () => {
    const { result, actions, onError } = setup();
    actions.exportSession.mockImplementation(() => { throw new Error("boom"); });
    act(() => { result.current.dispatch("/export"); });
    await waitFor(() => expect(onError).toHaveBeenCalledWith("SlashCommands.export", expect.any(Error)));
    expect(result.current.output?.tone).toBe("error");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/services/slash-commands/use-slash-commands.test.tsx`
Expected: FAIL — `Failed to resolve import "./use-slash-commands"`

- [ ] **Step 3: Write the implementation**

创建 `src/services/slash-commands/use-slash-commands.ts`：

```ts
import { useCallback, useMemo, useState } from "react";
import { SLASH_COMMANDS } from "./command-catalog";
import { findCommand, listCommands } from "./command-registry";
import { parseCommandInput } from "./parse-command";
import { slashCommandsEnabled } from "./command-availability";
import type { Session } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import type { CommandCapabilities, CommandContext, CommandOutput, SlashCommandNavigation } from "./types";

export type DispatchResult =
  | { kind: "message" }
  | { kind: "literal"; content: string }
  | { kind: "handled" };

const COMPLETION_PATTERN = /^\/([a-zA-Z][a-zA-Z0-9-]*)?$/;

function errorOutput(key: string, params?: Record<string, string | number>): CommandOutput {
  return { titleKey: "slash.error.title", tone: "error", messages: [{ key, params }] };
}

export function useSlashCommands(input: {
  session: Session | null;
  config: ChatConfig;
  isStreaming: boolean;
  actions: CommandContext["actions"];
  chat: CommandContext["chat"];
  navigate: SlashCommandNavigation;
  onError: (source: string, reason: unknown) => void;
}) {
  const { actions, chat, config, isStreaming, navigate, onError, session } = input;
  const [output, setOutput] = useState<CommandOutput | null>(null);
  const [suggestionQuery, setSuggestionQuery] = useState<string | null>(null);

  const enabled = slashCommandsEnabled(session);
  const capabilities = useMemo<CommandCapabilities>(
    () => ({ hasAssociatedPlan: navigate.openAssociatedPlan !== null }),
    [navigate.openAssociatedPlan],
  );

  const available = useMemo(
    () => (session && enabled ? listCommands(SLASH_COMMANDS, session, capabilities) : []),
    [capabilities, enabled, session],
  );

  const suggestions = useMemo(() => {
    if (suggestionQuery === null) return [];
    return available.filter((command) => command.name.startsWith(suggestionQuery)).slice(0, 8);
  }, [available, suggestionQuery]);

  /**
   * Called on every keystroke. It must never run anything — `dispatch` is the only entry point
   * allowed to have side effects, and conflating the two would fire a command per character.
   */
  const updateSuggestions = useCallback((draft: string) => {
    const completion = COMPLETION_PATTERN.exec(draft.trim());
    setSuggestionQuery(completion ? (completion[1] ?? "").toLowerCase() : null);
  }, []);

  const dispatch = useCallback((draft: string): DispatchResult => {
    setSuggestionQuery(null);

    const parsed = parseCommandInput(draft);
    if (parsed.kind === "literal") return parsed;
    if (parsed.kind === "message" || !session || !enabled) return { kind: "message" };

    const command = findCommand(SLASH_COMMANDS, parsed.name, session, capabilities);
    if (!command) {
      setOutput(errorOutput("slash.error.unknown", { command: parsed.name }));
      return { kind: "handled" };
    }

    const context: CommandContext = {
      session, config, isStreaming, chat, actions, navigate,
      listAvailableCommands: () => available,
    };

    // The caller needs a synchronous answer about whether the model should see this input, so the
    // handler's own result lands in state afterwards rather than being awaited here.
    void Promise.resolve()
      .then(() => command.run(context, parsed.args))
      .then((outcome) => setOutput(outcome.kind === "output" ? outcome.output : null))
      .catch((reason) => {
        onError(`SlashCommands.${command.name}`, reason);
        setOutput(errorOutput("slash.error.failed", { command: command.name }));
      });

    return { kind: "handled" };
  }, [actions, available, capabilities, chat, config, enabled, isStreaming, navigate, onError, session]);

  /** A command occupies the whole draft, so completing one replaces it rather than editing it. */
  const completeDraft = useCallback((name: string): string => `/${name} `, []);

  const dismissOutput = useCallback(() => setOutput(null), []);

  return { completeDraft, dismissOutput, dispatch, output, suggestionQuery, suggestions, updateSuggestions };
}
```

- [ ] **Step 4: Add the i18n keys**

五个 locale 各加两行：

- `en.json`: `"slash.error.unknown": "Unknown command /{{command}}. Try /help.",` `"slash.error.failed": "/{{command}} did not complete."`
- `zh-CN.json`: `"slash.error.unknown": "未知命令 /{{command}}，试试 /help。",` `"slash.error.failed": "/{{command}} 未能完成。"`
- `zh-TW.json`: `"slash.error.unknown": "未知指令 /{{command}}，請試試 /help。",` `"slash.error.failed": "/{{command}} 未能完成。"`
- `ja.json`: `"slash.error.unknown": "不明なコマンド /{{command}} です。/help をお試しください。",` `"slash.error.failed": "/{{command}} は完了しませんでした。"`
- `ko.json`: `"slash.error.unknown": "알 수 없는 명령 /{{command}} 입니다. /help 를 사용해 보세요.",` `"slash.error.failed": "/{{command}} 이(가) 완료되지 않았습니다."`

- [ ] **Step 5: Run test to verify it passes**

Run: `npx vitest run src/services/slash-commands/use-slash-commands.test.tsx src/i18n/slash-command-locales.test.ts`
Expected: PASS — 12 + 1 tests

- [ ] **Step 6: Commit**

```bash
git add src/services/slash-commands/ src/i18n/
git commit -m "feat: add slash command dispatch hook"
```

---

### Task 11: 接入 ChatInputBox

**Files:**
- Modify: `src/components/chat/ChatInputBox.tsx:70-96,106-123`
- Test: `src/components/chat/ChatInputBox.slash.test.tsx`

**Interfaces:**
- Consumes: `SlashCommandCompletion`（Task 9）、`SlashCommandOutput`（Task 8）、`SlashCommand`、`CommandOutput`
- Produces: `ChatInputBox` 新增四个可选 prop：`slashCommandOutput`、`slashCommandSuggestions`、`onDismissSlashCommandOutput`、`onSelectSlashCommand`

新增 prop 全部可选，`chat-experience.test.tsx` 等既有调用点不受影响。当前 188 行，本任务后约 225 行，仍低于 300。

- [ ] **Step 1: Write the failing test**

创建 `src/components/chat/ChatInputBox.slash.test.tsx`：

```tsx
import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithAppProviders } from "../../test/render";
import type { SlashCommand } from "../../services/slash-commands/types";
import { ChatInputBox } from "./ChatInputBox";

const command = (name: string): SlashCommand => ({
  name, category: "info", appliesTo: () => true, run: async () => ({ kind: "handled" }),
});

function renderBox(overrides: Partial<Parameters<typeof ChatInputBox>[0]> = {}) {
  return renderWithAppProviders(
    <ChatInputBox
      agents={[]} availableModes={["inherit"]} availableModels={[]} availableReasoning={["low"]}
      config={{ agentId: "onepiece", interactionMode: "api", executionMode: "inherit", streaming: true, thinking: false, longContext: false }}
      fileReferenceCandidates={[]} fileReferences={[]} isStreaming={false} value=""
      onAddFileReference={() => undefined} onChange={() => undefined} onClear={() => undefined}
      onConfigAgentChange={() => undefined} onConfigLongContextChange={() => undefined}
      onConfigModeChange={() => undefined} onConfigModelChange={() => undefined}
      onConfigProviderChange={() => undefined} onConfigReasoningChange={() => undefined}
      onConfigStreamingChange={() => undefined} onConfigThinkingChange={() => undefined}
      onRemoveFileReference={() => undefined} onStop={() => undefined} onSubmit={() => undefined}
      {...overrides}
    />,
  );
}

describe("ChatInputBox slash command surfaces", () => {
  it("renders neither surface by default", () => {
    renderBox();
    expect(screen.queryByTestId("slash-command-output")).not.toBeInTheDocument();
    expect(screen.queryByText("Commands")).not.toBeInTheDocument();
  });

  it("renders the completion dropdown from suggestions", () => {
    renderBox({ slashCommandSuggestions: [command("status")] });
    expect(screen.getByRole("button", { name: /\/status/ })).toBeInTheDocument();
  });

  it("reports the selected command", async () => {
    const onSelectSlashCommand = vi.fn();
    const { user } = renderBox({ slashCommandSuggestions: [command("usage")], onSelectSlashCommand });
    await user.click(screen.getByRole("button", { name: /\/usage/ }));
    expect(onSelectSlashCommand).toHaveBeenCalledWith("usage");
  });

  it("renders command output and forwards dismissal", async () => {
    const onDismissSlashCommandOutput = vi.fn();
    const { user } = renderBox({
      onDismissSlashCommandOutput,
      slashCommandOutput: { titleKey: "slash.output.applied", tone: "info", messages: [] },
    });
    expect(screen.getByTestId("slash-command-output")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Dismiss command output" }));
    expect(onDismissSlashCommandOutput).toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/components/chat/ChatInputBox.slash.test.tsx`
Expected: FAIL — 补全下拉与输出面板均未渲染

- [ ] **Step 3: Add the imports**

在 `src/components/chat/ChatInputBox.tsx` 的 import 区（第 8 行 `SeatMentionCompletion` 之后）加入：

```tsx
import { SlashCommandCompletion } from "./SlashCommandCompletion";
import { SlashCommandOutput } from "./SlashCommandOutput";
import type { CommandOutput, SlashCommand } from "../../services/slash-commands/types";
```

- [ ] **Step 4: Add the props**

在解构参数列表中，`participantMentions = [],` 之后加入：

```tsx
  slashCommandOutput = null,
  slashCommandSuggestions = [],
  onDismissSlashCommandOutput,
  onSelectSlashCommand,
```

在类型字面量中，`participantMentions?: SeatMentionOption[];` 之后加入：

```tsx
  slashCommandOutput?: CommandOutput | null;
  slashCommandSuggestions?: SlashCommand[];
  onDismissSlashCommandOutput?: () => void;
  onSelectSlashCommand?: (name: string) => void;
```

- [ ] **Step 5: Render both surfaces**

把第 112-123 行的悬浮面板块整体替换为：

```tsx
        <SlashCommandOutput onDismiss={onDismissSlashCommandOutput ?? (() => undefined)} output={slashCommandOutput} />
        {slashCommandSuggestions.length ? (
          <div className="ucd-panel absolute bottom-full left-0 z-20 mb-2 grid max-h-56 w-full gap-1 overflow-y-auto rounded-md p-1 text-xs shadow-lg">
            <SlashCommandCompletion onSelect={onSelectSlashCommand ?? (() => undefined)} options={slashCommandSuggestions} />
          </div>
        ) : null}
        {participantSuggestions.length || fileSuggestions.length ? (
          <div className="ucd-panel absolute bottom-full left-0 z-20 mb-2 grid max-h-56 w-full gap-1 overflow-y-auto rounded-md p-1 text-xs shadow-lg">
            <SeatMentionCompletion onSelect={selectParticipant} options={participantSuggestions} />
            {fileSuggestions.length ? <p className="px-2 py-1 text-[11px] font-semibold uppercase text-muted-foreground">{t("chat.completion.file")}</p> : null}
            {fileSuggestions.map((document) => (
              <button className="flex min-w-0 items-center gap-2 rounded px-2 py-1.5 text-left hover:bg-muted" key={document.path} onClick={() => selectReference(document)} type="button">
                <FileText className="h-3.5 w-3.5 shrink-0 text-primary" aria-hidden="true" />
                <span className="min-w-0 flex-1 truncate">{document.path}</span>
              </button>
            ))}
          </div>
        ) : null}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `npx vitest run src/components/chat/ChatInputBox.slash.test.tsx src/components/chat/chat-experience.test.tsx src/components/chat/associated-plan-navigation.test.tsx`
Expected: PASS — 新增 4 个用例通过，既有用例不回归

- [ ] **Step 7: Verify the file stayed under the line limit**

Run: `npx eslint src/components/chat/ChatInputBox.tsx`
Expected: 无输出（无 `max-lines` 报错）

- [ ] **Step 8: Commit**

```bash
git add src/components/chat/ChatInputBox.tsx src/components/chat/ChatInputBox.slash.test.tsx
git commit -m "feat: render slash command surfaces in the composer"
```

---

### Task 12: 接入 composer、main-layout 与页签

**Files:**
- Modify: `src/session-workspace/api-session-composer.tsx`（整文件替换）
- Modify: `src/session-workspace/session-tabs.tsx:48-52,63,85-89`
- Modify: `src/main-layout/main-layout.tsx:210-224,348`
- Test: `src/session-workspace/api-session-composer.test.tsx`

**Interfaces:**
- Consumes: `useSlashCommands`（Task 10）、`MainLayoutModel`
- Produces: `ApiSessionComposer` 新增可选 prop `navigation?: SlashCommandNavigation`；`SessionTabs` 新增可选 prop `requestedTabNonce?: number`

`SessionTabs` 的 `useEffect` 依赖 `[requestedTab, sessionId]`（第 89 行），同一个页签连点两次不会重新触发。加一个 nonce 参与依赖，`/logs` 才能在用户手动切回 chat 后再次生效。

`//` 转义送出字面文本时，`model.submit()` 读的是 `model.draft`，改写 draft 与提交必须跨一次渲染，因此用一个 pending 标志加 `useEffect` 完成。

- [ ] **Step 1: Write the failing test**

创建 `src/session-workspace/api-session-composer.test.tsx`：

```tsx
import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithAppProviders } from "../test/render";
import type { MainLayoutModel } from "../main-layout/use-main-layout-model";
import { ApiSessionComposer } from "./api-session-composer";

function model(overrides: Record<string, unknown> = {}) {
  const base = {
    activeSession: { id: "session-1", title: "S", agentId: "onepiece", interactionMode: "api", lifecycleState: "idle" },
    agents: [], draft: "", fileReferenceCandidates: [], fileReferences: [],
    isSending: false, isStreaming: false, messages: [],
    chatConfig: {
      availableAgents: [], availableModes: ["inherit"], availableModels: [], availableReasoning: ["low"],
      config: { agentId: "onepiece", interactionMode: "api", executionMode: "inherit", streaming: true, thinking: false, longContext: false },
      associatedPlanRun: null, changeAgent: vi.fn(), changeModel: vi.fn(), changeProvider: vi.fn(),
      setLongContext: vi.fn(), setReasoningDepth: vi.fn(), setSessionExecutionMode: vi.fn(),
      setStreaming: vi.fn(), setThinking: vi.fn(),
    },
    addFileReference: vi.fn(), removeFileReference: vi.fn(), exportSession: vi.fn(),
    setDraft: vi.fn(), stop: vi.fn(), submit: vi.fn(),
    ...overrides,
  };
  return base as unknown as MainLayoutModel;
}

describe("ApiSessionComposer slash dispatch", () => {
  it("sends ordinary prose to the model", async () => {
    const target = model({ draft: "hello" });
    const { user } = renderWithAppProviders(<ApiSessionComposer model={target} />);
    await user.click(screen.getByRole("button", { name: /Send/i }));
    expect(target.submit).toHaveBeenCalled();
  });

  it("runs a command instead of sending it", async () => {
    const target = model({ draft: "/mode execute" });
    const { user } = renderWithAppProviders(<ApiSessionComposer model={target} />);
    await user.click(screen.getByRole("button", { name: /Send/i }));
    expect(target.submit).not.toHaveBeenCalled();
    expect(target.chatConfig.setSessionExecutionMode).toHaveBeenCalledWith("execute");
    expect(target.setDraft).toHaveBeenCalledWith("");
  });

  it("does not intercept in a non-OnePiece session", async () => {
    const target = model({
      draft: "/mode execute",
      activeSession: { id: "s", title: "S", agentId: "claude-code", interactionMode: "cli", lifecycleState: "idle" },
    });
    const { user } = renderWithAppProviders(<ApiSessionComposer model={target} />);
    await user.click(screen.getByRole("button", { name: /Send/i }));
    expect(target.submit).toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/session-workspace/api-session-composer.test.tsx`
Expected: FAIL — `/mode execute` 被当成普通消息提交

- [ ] **Step 3: Rewrite the composer**

`src/session-workspace/api-session-composer.tsx` 整个文件替换为：

```tsx
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChatInputBox } from "../components/chat/ChatInputBox";
import type { MainLayoutModel } from "../main-layout/use-main-layout-model";
import { createChatOperationFailureEvent } from "../main-layout/chat-operation-failure";
import { useNotifications } from "../notifications/notification-provider";
import { useSessionRoles } from "../hooks/use-session-speakers";
import { agentService } from "../services/runtime-agent-client";
import { settingsService } from "../services/runtime-settings-client";
import { activeSeatsFromSession } from "../services/session-seats";
import { seatMentionOptions } from "../services/seat-mention-options";
import { canSendToSession } from "../services/session-admission";
import { useSlashCommands } from "../services/slash-commands/use-slash-commands";
import type { SlashCommandNavigation } from "../services/slash-commands/types";

const NO_NAVIGATION: SlashCommandNavigation = {
  openAssociatedPlan: null, openDestination: () => undefined, openSessionTab: () => undefined,
};

export function ApiSessionComposer({
  model,
  navigation,
  onOpenPlan,
}: {
  model: MainLayoutModel;
  navigation?: SlashCommandNavigation;
  onOpenPlan?: () => void;
}) {
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const isMultiSeat = Boolean(model.activeSession && activeSeatsFromSession(model.activeSession).length > 1);
  const roles = useSessionRoles(isMultiSeat);
  const participantMentions = seatMentionOptions(model.activeSession, model.agents, roles);
  const [pendingLiteralSend, setPendingLiteralSend] = useState(false);

  const slash = useSlashCommands({
    session: model.activeSession,
    config: model.chatConfig.config,
    isStreaming: model.isStreaming,
    chat: {
      setSessionExecutionMode: model.chatConfig.setSessionExecutionMode,
      setReasoningDepth: model.chatConfig.setReasoningDepth,
      setStreaming: model.chatConfig.setStreaming,
      setThinking: model.chatConfig.setThinking,
      setLongContext: model.chatConfig.setLongContext,
    },
    actions: {
      exportSession: model.exportSession,
      stop: model.stop,
      loadUsageSummary: async (sessionId) => {
        const summary = await agentService.getSessionUsageSummary(sessionId);
        return {
          totalTokens: summary.reported.totalTokens,
          inputTokens: summary.reported.inputTokens,
          outputTokens: summary.reported.outputTokens,
          responseCount: summary.responseCount,
        };
      },
    },
    navigate: navigation ?? { ...NO_NAVIGATION, openAssociatedPlan: onOpenPlan ?? null },
    onError: (source, reason) => {
      const event = createChatOperationFailureEvent(source, reason);
      notify({ type: "error", title: t("app.error.title"), message: event.message, scope: { kind: "global" } });
      void settingsService.reportClientLogEvent(event).catch(() => undefined);
    },
  });

  // `model.submit()` reads `model.draft`, so an unescaped literal has to land in state before the
  // send happens — one render apart, not one statement apart.
  useEffect(() => {
    if (!pendingLiteralSend) return;
    setPendingLiteralSend(false);
    model.submit();
  }, [model, pendingLiteralSend]);

  function submit() {
    const outcome = slash.dispatch(model.draft);
    if (outcome.kind === "handled") { model.setDraft(""); return; }
    if (outcome.kind === "literal") { model.setDraft(outcome.content); setPendingLiteralSend(true); return; }
    model.submit();
  }

  return (
    <ChatInputBox
      agents={model.chatConfig.availableAgents}
      availableModes={model.chatConfig.availableModes}
      availableModels={model.chatConfig.availableModels}
      availableReasoning={model.chatConfig.availableReasoning}
      config={model.chatConfig.config}
      disabled={!canSendToSession(model.activeSession) || model.isSending}
      fileReferenceCandidates={model.fileReferenceCandidates}
      fileReferences={model.fileReferences}
      isStreaming={model.isStreaming}
      lockRuntimeIdentity
      participantMentions={participantMentions}
      slashCommandOutput={slash.output}
      slashCommandSuggestions={slash.suggestions}
      onAddFileReference={model.addFileReference}
      onChange={(value) => { slash.updateSuggestions(value); model.setDraft(value); }}
      onClear={() => model.setDraft("")}
      onConfigAgentChange={model.chatConfig.changeAgent}
      onConfigLongContextChange={model.chatConfig.setLongContext}
      onConfigModeChange={model.chatConfig.setSessionExecutionMode}
      onConfigModelChange={model.chatConfig.changeModel}
      onConfigProviderChange={model.chatConfig.changeProvider}
      onConfigReasoningChange={model.chatConfig.setReasoningDepth}
      onConfigStreamingChange={model.chatConfig.setStreaming}
      onConfigThinkingChange={model.chatConfig.setThinking}
      onDismissSlashCommandOutput={slash.dismissOutput}
      onOpenPlan={model.chatConfig.associatedPlanRun ? onOpenPlan : undefined}
      onRemoveFileReference={model.removeFileReference}
      onSelectSlashCommand={(name) => model.setDraft(slash.completeDraft(name))}
      onStop={model.stop}
      onSubmit={submit}
      value={model.draft}
    />
  );
}
```

两处依赖需要同时落实：

1. **`agentService` 导入**：在文件顶部的 import 区加入 `import { agentService } from "../services/runtime-agent-client";`
2. **失败上报通道**：`use-main-layout-model.ts` 已有 `reportChatFailure`（第 66 行），但未导出。该文件是 298/300 行，**不要**往里加行。改在 composer 内自行上报——把上面 `onError` 的实现替换为：

```tsx
    onError: (source, reason) => {
      const event = createChatOperationFailureEvent(source, reason);
      notify({ type: "error", title: t("app.error.title"), message: event.message, scope: { kind: "global" } });
      void settingsService.reportClientLogEvent(event).catch(() => undefined);
    },
```

并相应补上这些 import：

```tsx
import { useTranslation } from "react-i18next";
import { createChatOperationFailureEvent } from "../main-layout/chat-operation-failure";
import { useNotifications } from "../notifications/notification-provider";
import { settingsService } from "../services/runtime-settings-client";
```

以及在组件体顶部取出 hook：

```tsx
  const { t } = useTranslation();
  const { notify } = useNotifications();
```

这与 `use-main-layout-model.ts:66-76` 的既有失败上报路径完全一致，只是调用点换到了 composer。

- [ ] **Step 4: Add the tab nonce**

`src/session-workspace/session-tabs.tsx`：在解构参数中 `requestedTab,` 之后加 `requestedTabNonce = 0,`；在类型字面量中 `requestedTab?: SessionTabId | null;` 之后加 `requestedTabNonce?: number;`；把第 85-89 行的 effect 改为：

```tsx
  useEffect(() => {
    if (!requestedTab) return;
    setMountedTabs((current) => new Set(current).add(requestedTab));
    setActiveTab(requestedTab);
    // The nonce lets the same tab be requested twice in a row — otherwise a second `/logs` after
    // the user manually returned to chat would be a no-op.
  }, [requestedTab, requestedTabNonce, sessionId]);
```

- [ ] **Step 5: Wire main-layout**

`src/main-layout/main-layout.tsx`：在第 117 行 `const [workBoardVisited, setWorkBoardVisited] = useState(false);` 之后加入：

```tsx
  const [slashTabRequest, setSlashTabRequest] = useState<{ tab: SessionTabId; nonce: number } | null>(null);
```

把第 210-212 行改为：

```tsx
  const requestedWorkspaceTab: SessionTabId | null = loopInspection
    ? loopInspection.target.surface === "usage" ? "chat" : loopInspection.target.surface
    : slashTabRequest?.tab ?? null;
```

把第 216-224 行的 `apiComposer` 改为：

```tsx
  const openAssociatedPlan = () => {
    const run = model.chatConfig.associatedPlanRun;
    if (!run) return;
    setPlanInspectionRunId(run.id);
    setPlanCenterVisited(true);
    setDestination("plans");
  };
  const apiComposer = !loopInspection && usesStructuredChat ? (
    <ApiSessionComposer
      model={model}
      navigation={{
        openAssociatedPlan: model.chatConfig.associatedPlanRun ? openAssociatedPlan : null,
        openDestination: (target) => {
          if (target === "todo-board") setWorkBoardVisited(true);
          if (target === "plans") setPlanCenterVisited(true);
          setDestination(target);
        },
        openSessionTab: (tab) => setSlashTabRequest((current) => ({ tab, nonce: (current?.nonce ?? 0) + 1 })),
      }}
      onOpenPlan={openAssociatedPlan}
    />
  ) : null;
```

在第 348 行 `requestedTab={requestedWorkspaceTab}` 之后加入：

```tsx
                  requestedTabNonce={slashTabRequest?.nonce ?? 0}
```

若 `setLoopCenterVisited` / `setPlanCenterVisited` 的实际标识符名与此处不符，照抄该文件第 252-260 行既有写法中的名字。

- [ ] **Step 6: Run test to verify it passes**

Run: `npx vitest run src/session-workspace/api-session-composer.test.tsx src/session-workspace/session-workspace-components.test.tsx`
Expected: PASS

- [ ] **Step 7: Check line limits on every touched file**

Run: `npx eslint src/session-workspace/api-session-composer.tsx src/session-workspace/session-tabs.tsx src/main-layout/main-layout.tsx src/main-layout/use-main-layout-model.ts`
Expected: 无输出。`use-main-layout-model.ts` 若报 `max-lines`，改用 Step 3 末尾给出的备选方案

- [ ] **Step 8: Commit**

```bash
git add src/session-workspace/ src/main-layout/ src/services/slash-commands/
git commit -m "feat: dispatch slash commands from the OnePiece composer"
```

---

### Task 13: 端到端验证与全量校验

**Files:**
- Create: `tests/e2e/slash-commands.spec.ts`
- Test: 全仓库

**Interfaces:**
- Consumes: 前 12 个任务的全部产出
- Produces: 无（验证任务）

- [ ] **Step 1: Find the existing e2e conventions**

Run: `ls tests/e2e/ && head -30 tests/e2e/*.spec.ts | head -50`
Expected: 看到既有 spec 的 `test.describe` 结构、如何进入会话、用的 selector 约定。**照抄该文件的启动与导航方式**，不要自行发明

- [ ] **Step 2: Write the e2e spec**

创建 `tests/e2e/slash-commands.spec.ts`。骨架如下，其中进入 OnePiece 会话的步骤替换为 Step 1 中看到的既有写法：

```ts
import { expect, test } from "@playwright/test";

test.describe("OnePiece slash commands", () => {
  test("/help lists commands without sending a message", async ({ page }) => {
    // <照抄既有 spec 的打开应用 + 进入 OnePiece 会话步骤>
    const composer = page.getByTestId("wechat-style-composer");
    const input = composer.getByRole("textbox");

    await input.fill("/help");
    await input.press("Enter");

    await expect(page.getByTestId("slash-command-output")).toBeVisible();
    await expect(page.getByTestId("slash-command-output")).toContainText("/status");
    await expect(input).toHaveValue("");
  });

  test("an unknown command is reported and not sent", async ({ page }) => {
    // <同上的进入步骤>
    const input = page.getByTestId("wechat-style-composer").getByRole("textbox");
    await input.fill("/definitelynotacommand");
    await input.press("Enter");

    const output = page.getByTestId("slash-command-output");
    await expect(output).toHaveAttribute("data-tone", "error");
  });

  test("typing a slash offers completions", async ({ page }) => {
    // <同上的进入步骤>
    const input = page.getByTestId("wechat-style-composer").getByRole("textbox");
    await input.fill("/st");
    await expect(page.getByRole("button", { name: /\/status/ })).toBeVisible();
  });
});
```

- [ ] **Step 3: Run the e2e spec**

本机的 SOCKS5 代理会让 Playwright 崩溃，且它可能复用别的 worktree 的 dev server，因此必须同时清代理并固定端口：

Run: `env -u all_proxy -u ALL_PROXY PLAYWRIGHT_PORT=5199 npx playwright test tests/e2e/slash-commands.spec.ts`
Expected: 3 passed

- [ ] **Step 4: Run the full frontend verification**

```bash
npm run lint:ci
npm run test
npm run build
```

Expected: lint 无警告；测试全绿（`skills-page-interactions.test.tsx` 与 `skill-overlay-reconciliation.test.tsx` 在满负载下偶发 10s 超时，隔离重跑即过，非本次回归）；build 成功

- [ ] **Step 5: Run the Rust verification**

本阶段未改 Rust，但 CI 恒跑，需确认未被牵连：

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: 全部通过

- [ ] **Step 6: Run the remaining CI gates**

```bash
npm run test:coverage
npm run contracts:check
npm run docs:check
openspec validate --specs --strict
```

Expected: 全部通过

- [ ] **Step 7: Commit**

```bash
git add tests/e2e/slash-commands.spec.ts
git commit -m "test: add slash command end-to-end coverage"
```

---

## 落地前置条件

AGENTS.md 要求任何新功能先在 `openspec/changes/` 下起 proposal 并通过 `openspec validate <change-name> --strict`，然后才动代码。**执行本计划前必须先完成该 proposal**——本设计文档与实施计划不能替代它。建议的 change 名：`add-onepiece-slash-commands`。

## Self-Review 记录

- **Spec coverage**：设计文档中标注"后端成本：无"的命令全部落到 Task 4-7；补全与输出面板落到 Task 8-9；拦截点与扩展缝落到 Task 2、10、12；错误处理的六条落到 Task 4（参数非法）、5（`/stop` 非流式）、10（未知命令、handler 抛错）、1（`//` 转义）；`appliesTo` 过滤落到 Task 3。`/clear` 与 `/compact` 按设计属于第二阶段，本计划明确排除
- **已知缺口**：设计文档提到"`isStreaming` 时禁用 `/clear` `/compact`"——这两条命令本阶段不存在，该约束随第二阶段落地
- **修正 1（按键即执行命令）**：初稿把 `slash.dispatch` 接到了 `onChange`，那会在每敲一个字符时执行一次命令。已拆成两个入口——`updateSuggestions`（按键调用，只刷新补全）与 `dispatch`（提交调用，唯一有副作用的入口），并在 Task 10 补了 `never executes a command from updateSuggestions` 回归用例
- **修正 2（不存在的上报方法）**：初稿的 `onError` 调用了 `model.reportSlashCommandFailure`，但 `use-main-layout-model.ts` 并未导出该方法，且该文件是 298/300 行、无豁免、不能加行。已改为在 composer 内复用 `createChatOperationFailureEvent` + `notify` + `reportClientLogEvent`，与 `use-main-layout-model.ts:66-76` 的既有路径一致
- **类型一致性**：`SlashCommand`、`CommandContext`、`CommandOutput`、`CommandMessage`、`SlashCommandNavigation`、`SlashCommandDestination`、`DispatchResult` 在 Task 3 与 Task 10 定义一次，后续任务全部按同名引用；`loadUsageSummary` 的返回结构在 Task 3（类型）、Task 5（消费）、Task 12（实现）三处一致
- **修正 3（模块级可变状态）**：初稿让 `/plan` 的可用性依赖 `navigation-commands.ts` 里的模块级 `associatedPlanAvailable`，由 hook 在**渲染期**调 `setAssociatedPlanAvailability` 同步。三重问题：模块级可变状态、渲染期副作用、测试相互污染。已改为 `appliesTo(session, capabilities)` 显式接收 `CommandCapabilities`；`completeDraft` 也顺带去掉了那个未使用的 `draft` 参数。注意措辞：双参数**并不能让类型系统强制纯粹性**——TS 的函数类型拦不住闭包捕获模块级变量。它真正做到的是把唯一已知的非 session 事实显式供上，从而移除了伸手拿全局变量的理由。纯粹性因此是纪律，写命令时仍需自觉
