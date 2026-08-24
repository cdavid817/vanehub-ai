import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, readFile, symlink, unlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];
const stamp = Date.now().toString(36);

/**
 * A LARGE-granularity requirement through a full three-seat relay on the product's own source
 * tree: "settings → about gains a 复制诊断信息 feature" — a new pure service module, a page
 * change, five locale files, and a new unit test. Four mandated agent turns (架构师 → 实现者 →
 * 代码审查 → 实现者 rework) exercise the depth-4 chain and the re-dispatch of a seat that has
 * already spoken. `VANEHUB_LONGRUN_ADJUDICATION` picks the relay discipline: `human` (default)
 * makes the implementer stop on a blocking `@用户 handoff` — the harness, playing the human,
 * verifies the round actually pauses and then routes the review with a line-leading mention —
 * while `auto` lets the seats relay unattended end to end. Long enough (typically 15-30 minutes
 * of real model work) to surface duration problems: the 4000-char shared-thread injection
 * budget, per-turn timeouts, oversized provider records, and coordinator lifetime.
 *
 * The judge is the project's own toolchain run by this harness after the round — vitest on the
 * new test file, `tsc --noEmit`, eslint over the changed files, and locale-key greps — with the
 * criteria quoted verbatim in the requirement message.
 *
 * Host findings carried over: codex-cli takes the words-only architect seat; claude-code and
 * opencode run the `trusted` template (edits auto-approved, commands not), and every acting seat
 * is told to write files and never run commands.
 */
// Reviewer-seat history on this host: opencode's model gateway 400s on every second step
// (`input.status` missing, isRetryable:false — an opencode↔endpoint break, not VaneHub), and
// codex's bwrap command sandbox intermittently cannot start inside the app
// (`bwrap: loopback: RTM_NEWADDR Operation not permitted`), leaving it unable to read anything.
// claude-code takes the reviewer seat: its file reads do not depend on bwrap. Same model family
// as the implementer — `requireDifferentFamily` is presentational in the frontend, not enforced
// at seat assignment — which this rehearsal accepts in exchange for a review that can actually
// see the code.
const TRIO = ["codex-cli", "claude-code", "claude-code"];
const TRUSTED_SEATS = ["claude-code"];
// Two relay disciplines, chosen per run: with `human` (the default) the implementer must stop on
// a blocking `@用户 handoff` and the harness — playing the human — verifies the pause and routes
// the review; with `auto` the implementer hands off straight to the reviewer and the round runs
// unattended end to end. The product itself needs no switch — the discipline lives entirely in
// the requirement message, which is exactly how a real dispatcher would choose a mode.
const HUMAN_ADJUDICATION = globalThis.process.env.VANEHUB_LONGRUN_ADJUDICATION !== "auto";
const BUILTIN_ROLES = ["builtin-architect", "builtin-implementer", "builtin-reviewer"];
const STAGE_BUDGET_MS = 12 * 60 * 1000;
const LOCALES = ["en", "zh-CN", "zh-TW", "ja", "ko"];
const NEW_KEYS = ["about.diagnostics.copy", "about.diagnostics.copied"];

const specDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(specDir, "..", "..", "..");

async function settleOperation(operation, timeoutMsg) {
  const settled = await globalThis.browser.waitUntil(async () => {
    const status = await invoke(
      ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
      operation.id,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
  }, { timeout: 90_000, timeoutMsg });
  assert.equal(settled.status, "succeeded", settled.error ?? timeoutMsg);
  return settled;
}

async function createTargetWorktree() {
  const dest = join(await mkdtemp(join(tmpdir(), "vanehub-agent-longrun-")), "repo");
  const branch = `agents/copy-diagnostics-${stamp}`;
  await run("git", ["worktree", "add", dest, "-b", branch, "main"], { cwd: repoRoot });
  return { dest, branch };
}

let target = null;
const flow = { session: null, seats: [], handles: [], failed: false, ordinal: 0, retried: false };

const listMessages = (sessionId) => invoke(({ core }, id) => core.invoke("list_messages", {
  sessionId: id,
  limit: null,
  beforeId: null,
}), sessionId);

const assistantsOf = (messages) => messages.filter((message) => message.role === "assistant");

async function dumpDiagnostics(tag) {
  const resultDir = globalThis.process.env.VANEHUB_DESKTOP_RESULT_DIR;
  if (!resultDir || !flow.session) return;
  const capture = {};
  capture.messages = await listMessages(flow.session.id).catch((error) => String(error));
  capture.details = await invoke(
    ({ core }, id) => core.invoke("get_session_details", { sessionId: id }),
    flow.session.id,
  ).catch((error) => String(error));
  capture.logs = await invoke(({ core }, id) => core.invoke("list_session_logs", {
    input: { sessionId: id, levels: ["error", "warn", "info", "debug"], search: "", cursor: null, limit: 500 },
  }), flow.session.id).catch((error) => String(error));
  await writeFile(
    join(resultDir, `longrun-diagnostics-${tag}.json`),
    `${JSON.stringify(capture, null, 2)}\n`,
  ).catch(() => {});
}

async function turnAt(stage, ordinal, seat, handle) {
  const row = await globalThis.browser.waitUntil(async () => {
    const rows = assistantsOf(await listMessages(flow.session.id).catch(() => []));
    return rows.length > ordinal ? rows[ordinal] : false;
  }, {
    timeout: STAGE_BUDGET_MS,
    interval: 5_000,
    timeoutMsg: `turn ${ordinal} (${handle}) was never dispatched`,
  }).catch(() => null);
  if (!row) {
    const rows = assistantsOf(await listMessages(flow.session.id).catch(() => []));
    const previous = rows[ordinal - 1];
    if (previous && !(previous.content ?? "").includes(`@${handle}`)) {
      blocked.push(`${stage}: the previous turn did not emit @${handle}, so the relay had `
        + `nothing to act on (it replied: ${JSON.stringify((previous.content ?? "").slice(0, 200))})`);
      await dumpDiagnostics(`${stage}-no-mention`);
      return null;
    }
    flow.failed = true;
    await dumpDiagnostics(`${stage}-never-dispatched`);
    assert.fail(`turn ${ordinal} for @${handle} (${seat.agentId}) was never dispatched`);
  }
  assert.equal(row.speakerSeatId, seat.seatId, `turn ${ordinal} belongs to a seat other than @${handle}`);
  const settled = await globalThis.browser.waitUntil(async () => {
    const current = assistantsOf(await listMessages(flow.session.id).catch(() => []))[ordinal];
    return ["completed", "failed", "cancelled"].includes(current?.status) ? current : false;
  }, {
    timeout: STAGE_BUDGET_MS,
    interval: 5_000,
    timeoutMsg: `turn ${ordinal} (${handle}) never settled`,
  }).catch(() => null);
  if (!settled || settled.status !== "completed" || !settled.content.trim()) {
    blocked.push(`${stage}: the ${seat.agentId} turn ended ${settled?.status ?? "never"} `
      + `(${JSON.stringify((settled?.error ?? settled?.content ?? "").slice(0, 200))}); that `
      + "seat's runtime failed, not the relay");
    await dumpDiagnostics(`${stage}-turn-${ordinal}`);
    return null;
  }
  return settled;
}

function bail(context) {
  flow.failed = true;
  context.skip();
}

/** The harness speaking as the human: one message into the session. */
async function sendHuman(content) {
  await invoke(({ core }, payload) => core.invoke("send_message", payload), {
    sessionId: flow.session.id,
    content,
    config: {
      agentId: flow.session.agentId,
      interactionMode: "cli",
      executionMode: "inherit",
      providerId: null,
      modelId: null,
      reasoningDepth: null,
      streaming: true,
      thinking: false,
      longContext: false,
    },
    fileReferences: null,
  });
}

/**
 * A seat may end on a blocking handoff the script did not plan for — a reviewer once escalated
 * an OpenSpec-process question to the human, and per the routing rules that pause wins over the
 * teammate mention in the same reply. Answer it and hand the turn to `handle` so the relay
 * continues instead of timing out.
 */
async function answerUnplannedHandoff(handle) {
  const rows = assistantsOf(await listMessages(flow.session.id).catch(() => []));
  const last = rows[flow.ordinal - 1];
  if (!/^\s*@用户\s+handoff/mu.test(last?.content ?? "")) return;
  blocked.push(`unplanned handoff answered: routed to @${handle}`);
  await sendHuman(`@${handle} 人类裁决:流程类事项(如 OpenSpec 提案)本轮豁免,由外部系统另行处理;`
    + "请按审查中标记为必须修的意见继续返工,不要再等待人类。");
}

/** A stage snapshot beside the wdio evidence, so the visible run leaves visible proof. */
async function snap(tag) {
  const resultDir = globalThis.process.env.VANEHUB_DESKTOP_RESULT_DIR;
  if (!resultDir) return;
  await globalThis.browser
    .saveScreenshot(join(resultDir, "screenshots", `stage-${tag}.png`))
    .catch(() => {});
}

/**
 * Puts the fixture session on screen. The session was created behind the UI's back, and the
 * workspace route only mounts a session the UI's own list already contains — a reload is how the
 * UI learns about it (same maneuver as ui-multi-agent.e2e.mjs). After this, every turn of the
 * relay renders live in the window instead of happening invisibly over IPC.
 */
async function openSessionInUi() {
  await globalThis.browser.refresh();
  const root = await globalThis.$("#root");
  await root.waitForExist({ timeout: 120_000 });
  await globalThis.browser.waitUntil(
    async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
    { timeout: 120_000, timeoutMsg: "React bootstrap did not come back after the reload." },
  );
  await globalThis.browser.execute((target) => {
    globalThis.history.pushState({}, "", target);
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  }, `/workspace/sessions/${encodeURIComponent(flow.session.id)}`);
  await (await globalThis.$('[aria-controls="session-tab-panel-chat"]')).waitForExist({ timeout: 30_000 });
  await globalThis.browser.waitUntil(async () => {
    const active = await invoke(({ core }) => core.invoke("get_active_session"));
    return active?.id === flow.session.id;
  }, { timeout: 30_000, timeoutMsg: "The app never switched to the longrun session." });
}

/**
 * One seat turn at the thread's cursor, with a single human-style retry: when a seat's runtime
 * fails transiently (a provider error event mid-stream — observed once with opencode/glm), a
 * person would tell it to try again rather than abandon a 20-minute round. The retry is itself a
 * long-run behavior worth exercising: the failed row stays in the thread and the relay resumes.
 */
async function relayTurn(stage, seat, handle) {
  let attempt = await turnAt(stage, flow.ordinal, seat, handle);
  if (attempt) {
    flow.ordinal += 1;
    return attempt;
  }
  const rows = assistantsOf(await listMessages(flow.session.id).catch(() => []));
  const last = rows[flow.ordinal];
  if (!last || !["failed", "cancelled"].includes(last.status)) return null;
  flow.retried = true;
  blocked.push(`${stage}: retrying after a ${last.status} ${seat.agentId} turn`);
  await invoke(({ core }, payload) => core.invoke("send_message", payload), {
    sessionId: flow.session.id,
    content: `@${handle} 上一轮执行失败,请重新执行你负责的步骤,遵守原始分工与结尾提及规则。`,
    config: {
      agentId: flow.session.agentId,
      interactionMode: "cli",
      executionMode: "inherit",
      providerId: null,
      modelId: null,
      reasoningDepth: null,
      streaming: true,
      thinking: false,
      longContext: false,
    },
    fileReferences: null,
  });
  flow.ordinal += 1;
  attempt = await turnAt(`${stage}-retry`, flow.ordinal, seat, handle);
  if (!attempt) return null;
  flow.ordinal += 1;
  return attempt;
}

/** Toolchain commands need node_modules; borrow this checkout's via a symlink. */
async function withNodeModules(fn) {
  const link = join(target.dest, "node_modules");
  await symlink(join(repoRoot, "node_modules"), link).catch(() => {});
  try {
    return await fn();
  } finally {
    await unlink(link).catch(() => {});
  }
}

async function tool(command, args, timeout) {
  try {
    const { stdout, stderr } = await run(command, args, { cwd: target.dest, timeout });
    return { ok: true, output: `${stdout}\n${stderr}` };
  } catch (error) {
    return { ok: false, output: `${error.stdout ?? ""}\n${error.stderr ?? error.message}` };
  }
}

/** The verbatim criteria: project toolchain plus structural greps. */
async function judge() {
  const failures = [];
  const { stdout: porcelain } = await run("git", ["status", "--porcelain"], { cwd: target.dest });
  if (!porcelain.includes("src/services/about-diagnostics.ts")) {
    failures.push("src/services/about-diagnostics.ts was not created");
  }
  if (!porcelain.includes("src/settings/pages/about-page.tsx")) {
    failures.push("src/settings/pages/about-page.tsx was not modified");
  }
  for (const locale of LOCALES) {
    const text = await readFile(join(target.dest, `src/i18n/locales/${locale}.json`), "utf8")
      .catch(() => "");
    for (const key of NEW_KEYS) {
      if (!text.includes(`"${key}"`)) failures.push(`${locale}.json is missing ${key}`);
    }
  }
  await withNodeModules(async () => {
    const vitest = await tool("npx", [
      "vitest", "run",
      "src/services/about-diagnostics.test.ts",
      "src/settings/pages/about-page.test.tsx",
    ], 240_000);
    if (!vitest.ok) failures.push(`vitest failed:\n${vitest.output.slice(-1_800)}`);
    const tsc = await tool("npx", ["tsc", "--noEmit"], 300_000);
    if (!tsc.ok) failures.push(`tsc --noEmit failed:\n${tsc.output.slice(-1_200)}`);
    const { stdout: changed } = await run(
      "git", ["diff", "--name-only", "main"], { cwd: target.dest },
    );
    const lintable = changed.split("\n").filter((file) => /\.(ts|tsx)$/u.test(file));
    if (lintable.length > 0) {
      const eslint = await tool("npx", ["eslint", "--no-warn-ignored", ...lintable], 240_000);
      if (!eslint.ok) failures.push(`eslint failed:\n${eslint.output.slice(-1_200)}`);
    }
  });
  return failures;
}

globalThis.describe("VaneHub AI multi-Agent develops a large-granularity feature", () => {
  globalThis.before(async function prepare() {
    this.timeout(180_000);
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    target = await createTargetWorktree();
  });

  globalThis.it("seats the trio and the 架构师 designs the feature", async function architect() {
    this.timeout(STAGE_BUDGET_MS * 2 + 240_000);
    const lineupAgents = await globalThis.browser.waitUntil(async () => {
      const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }))
        .catch(() => null);
      if (!Array.isArray(agents)) return false;
      const lineup = TRIO.map((id) => agents.find((agent) => agent.id === id
        && agent.availabilityState === "available"
        && agent.supportedInteractionModes.includes("cli")));
      return lineup.every(Boolean) ? lineup : false;
    }, { timeout: 120_000, interval: 3_000, timeoutMsg: "the trio never became available" })
      .catch(() => null);
    if (!lineupAgents) {
      blocked.push(`longrun flow: ${TRIO.join(", ")} not all installed/usable on this host`);
      bail(this);
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const seatRoles = BUILTIN_ROLES.map((id) => roles.find((role) => role.id === id)).filter(Boolean);
    if (seatRoles.length < 3) {
      blocked.push("longrun flow: fewer than three built-in expert roles are available");
      bail(this);
    }
    for (const agentId of TRUSTED_SEATS) {
      await invoke(({ core }, input) => core.invoke("apply_policy_template", { input }), {
        agentId,
        template: "trusted",
      });
    }

    const title = `multiagent-longrun-${stamp}`;
    const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
      agentId: lineupAgents[0].id,
      interactionMode: "cli",
      title,
      folder: target.dest,
      projectPath: target.dest,
      remoteWorkspace: null,
      worktree: null,
    });
    await settleOperation(operation, "Creating the longrun session never settled.");
    const created = await globalThis.browser.waitUntil(async () => {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === title) ?? false;
    }, { timeout: 30_000, timeoutMsg: "The longrun session was not created." });

    flow.session = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
      sessionId: created.id,
      expectedUpdatedAt: created.updatedAt,
      seats: lineupAgents.map((agent, index) => ({ agentId: agent.id, roleId: seatRoles[index].id })),
    });
    flow.seats = flow.session.seats.filter((seat) => !seat.leftAt);
    flow.handles = seatRoles.map((role) => role.displayName.split(/\s+/u).filter(Boolean).join("-"));
    assert.deepEqual(flow.seats.map((seat) => seat.agentId), TRIO);

    // Pin the UI language before it goes on screen, then put the session's chat view up so the
    // whole relay scrolls live in the window.
    await invoke(({ core }) => core.invoke("save_setting", {
      input: { key: "applicationLanguage", value: "zh-CN" },
    }));
    await openSessionInUi();
    await snap("0-session-open");

    await invoke(({ core }, payload) => core.invoke("send_message", payload), {
      sessionId: flow.session.id,
      content: [
        `@${flow.handles[0]} 大颗粒真实需求:当前仓库是 VaneHub AI 桌面应用源码。为「设置 → 关于」页新增`,
        "「复制诊断信息」功能,分工与纪律必须严格遵守:",
        "",
        "功能要求:",
        "- 新建 src/services/about-diagnostics.ts:导出纯函数 buildAboutDiagnostics,返回多行诊断文本",
        "  (至少含产品名、当前版本、仓库地址;版本等取值通过参数传入或复用 about-service 的导出,",
        "  函数本身不得直接访问 Tauri API 或 window,保证可单测)。",
        "- 修改 src/settings/pages/about-page.tsx:新增一个复制按钮,点击后用 navigator.clipboard",
        "  写入 buildAboutDiagnostics 的结果,复制成功后按钮短暂显示「已复制」态;按钮文案走 i18n。",
        "- 5 个语言文件 src/i18n/locales/{en,zh-CN,zh-TW,ja,ko}.json 各新增两个键:",
        "  about.diagnostics.copy 与 about.diagnostics.copied。",
        "- 新建单测 src/services/about-diagnostics.test.ts 覆盖纯函数的正常与边界情况。",
        "",
        "分工(外部系统会核对轮次顺序):",
        "1. 架构师(你):只做设计,不改代码。给出文件清单、buildAboutDiagnostics 的完整签名与返回格式、",
        "   about-page 的接入方式与「已复制」状态的实现方案、测试用例要点,并说明取舍与被否决的替代方案。",
        `   设计写完后,最后单独起一行、只写 @${flow.handles[1]}。`,
        "2. 实现者:你有仓库文件写权限,没有命令执行权限。按设计创建/修改全部文件,不要征求确认,",
        "   也不要运行任何命令——lint 与测试由外部系统执行。注意 about-page.tsx 全文件不得超过 300 行。",
        ...(HUMAN_ADJUDICATION
          ? [
            "   改完说明动了哪些文件,然后最后单独起一行、只写 @用户 handoff ——把是否进入审查交给",
            "   人类决定,在人类回复之前不要点名任何席位。",
            "3. 代码审查(仅在人类点名你之后开始):读取全部改动文件,必须提出恰好一条需要实现者修改的",
          ]
          : [
            `   改完说明动了哪些文件,然后最后单独起一行、只写 @${flow.handles[2]}。`,
            "3. 代码审查:读取全部改动文件,必须提出恰好一条需要实现者修改的",
          ]),
        "   具体意见(引用文件与行内内容,例如遗漏的边界测试、错误处理或 i18n 遗漏),不要运行命令,",
        "   只用文件读取工具。然后,",
        `   然后最后单独起一行、只写 @${flow.handles[1]},把工作交回实现者。`,
        "4. 实现者(第二轮):按审查意见直接修改文件,不要运行命令,",
        "   然后最后单独起一行、只写 @用户 done,结束本轮。",
        "",
        "外部系统的验收标准(逐字执行):a) about-diagnostics.ts 与其单测存在且 vitest 通过;",
        "b) npx tsc --noEmit 通过;c) eslint 对全部改动 ts/tsx 文件通过(含 max-lines 300 行规则);",
        "d) 5 个语言文件都含上述两个键;e) about-page.tsx 被修改且引用 buildAboutDiagnostics。",
      ].join("\n"),
      config: {
        agentId: flow.session.agentId,
        interactionMode: "cli",
        executionMode: "inherit",
        providerId: null,
        modelId: null,
        reasoningDepth: null,
        streaming: true,
        thinking: false,
        longContext: false,
      },
      fileReferences: null,
    });

    const turn = await relayTurn("architect", flow.seats[0], flow.handles[0]);
    await snap("1-architect");
    if (!turn) bail(this);
    assert.ok(
      /buildAboutDiagnostics/u.test(turn.content),
      `the design never names buildAboutDiagnostics (got ${JSON.stringify(turn.content.slice(0, 200))})`,
    );

    // The transcript on screen must show this backend-originated conversation — the requirement
    // message and the seat's reply were injected over IPC, not typed into the composer
    // (fix-chat-transcript-backend-message-updates). message-speaker is MessageItem's speaker
    // label testid.
    await globalThis.browser.waitUntil(async () => {
      const speakers = await globalThis.$$('[data-testid="message-speaker"]');
      return speakers.length >= 1;
    }, {
      timeout: 30_000,
      interval: 2_000,
      timeoutMsg: "the open transcript never rendered the backend-originated turns",
    });
  });

  globalThis.it("the 实现者 builds the feature across files", async function implement() {
    this.timeout(STAGE_BUDGET_MS * 2 + 120_000);
    if (flow.failed) this.skip();
    const turn = await relayTurn("implementer", flow.seats[1], flow.handles[1]);
    await snap("2-implementer");
    if (!turn) bail(this);
  });

  globalThis.it("the blocking handoff pauses the round until the human routes the review", async function adjudicate() {
    this.timeout(5 * 60 * 1000);
    if (flow.failed) this.skip();
    // In auto mode the implementer hands off straight to the reviewer; there is no human stage.
    if (!HUMAN_ADJUDICATION) this.skip();
    const rows = assistantsOf(await listMessages(flow.session.id));
    const implementerTurn = rows[flow.ordinal - 1];
    if (!/^\s*@用户\s+handoff/mu.test(implementerTurn?.content ?? "")) {
      blocked.push("adjudicate: the implementer did not end with a line-leading @用户 handoff "
        + `(replied: ${JSON.stringify((implementerTurn?.content ?? "").slice(0, 160))}); the `
        + "relay continues without a human pause this round");
      this.skip();
    }
    // The pause, asserted as an absence over a window: nobody may be dispatched while the round
    // waits on the human (same technique as domain-multi-agent-human-decision.e2e.mjs).
    const before = assistantsOf(await listMessages(flow.session.id)).length;
    const dispatchedAnyway = await globalThis.browser.waitUntil(async () => {
      const current = assistantsOf(await listMessages(flow.session.id).catch(() => []));
      return current.length > before ? current[before] : false;
    }, { timeout: 30_000, interval: 3_000, timeoutMsg: "" }).catch(() => null);
    assert.equal(
      dispatchedAnyway,
      null,
      "the round continued while it was meant to be waiting on the human "
        + `(a turn belongs to ${JSON.stringify(dispatchedAnyway?.speakerSeatId ?? null)})`,
    );
    await snap("2b-handoff-pause");

    // The human's adjudication: a line-leading mention hands the turn to the reviewer seat.
    await sendHuman(`@${flow.handles[2]} 人类裁决:实现说明已确认,请按原始分工第3条开始审查。`);
  });

  globalThis.it("the 代码审查 sends one concrete rework item back", async function review() {
    this.timeout(STAGE_BUDGET_MS * 2 + 120_000);
    if (flow.failed) this.skip();
    const turn = await relayTurn("reviewer", flow.seats[2], flow.handles[2]);
    await snap("3-reviewer");
    if (!turn) bail(this);
    assert.ok(
      /diagnostics|about|test|i18n|copy/iu.test(turn.content),
      `the review does not reference the feature's files (got ${JSON.stringify(turn.content.slice(0, 200))})`,
    );
  });

  globalThis.it("the rework lands and the project's own toolchain accepts the feature", async function rework() {
    this.timeout(STAGE_BUDGET_MS * 2 + 15 * 60 * 1000);
    if (flow.failed) this.skip();
    await answerUnplannedHandoff(flow.handles[1]);
    const turn = await relayTurn("rework", flow.seats[1], flow.handles[1]);
    await snap("4-rework");
    if (!turn) bail(this);

    await globalThis.browser.waitUntil(async () => {
      const current = assistantsOf(await listMessages(flow.session.id).catch(() => []));
      return current.every((row) => ["completed", "failed", "cancelled"].includes(row.status))
        ? current.length
        : false;
    }, {
      timeout: STAGE_BUDGET_MS,
      interval: 5_000,
      timeoutMsg: "the round never settled: some turn is still streaming",
    });

    const rows = assistantsOf(await listMessages(flow.session.id));
    // Failed-and-retried turns stay in the thread, so the mandated relay is asserted over the
    // COMPLETED turns' prefix rather than raw row positions.
    const completedSpeakers = rows
      .filter((row) => row.status === "completed")
      .map((row) => row.speakerSeatId);
    assert.deepEqual(
      completedSpeakers.slice(0, 4),
      [flow.seats[0], flow.seats[1], flow.seats[2], flow.seats[1]].map((seat) => seat.seatId),
      "the completed turns do not begin with the relay 架构师 → 实现者 → 代码审查 → 实现者",
    );
    const known = new Set(flow.seats.map((seat) => seat.seatId));
    assert.ok(
      rows.every((row) => known.has(row.speakerSeatId)),
      "a turn in the thread belongs to no seated participant",
    );

    await snap("5-round-complete");
    const failures = await judge();
    assert.deepEqual(failures, [], `acceptance criteria failed:\n  ${failures.join("\n  ")}`);

    await run("git", ["add", "-A"], { cwd: target.dest });
    await run("git", [
      "-c", "user.name=VaneHub Multi-Agent E2E",
      "-c", "user.email=desktop-e2e@example.invalid",
      "commit", "-m", "feat: add copy-diagnostics action to Settings About page\n\n"
        + "Produced by a VaneHub AI three-seat multi-agent session (codex architect,\n"
        + "claude implementer, opencode reviewer); accepted by the project toolchain\n"
        + "run mechanically by the desktop e2e harness.",
    ], { cwd: target.dest });

    const resultDir = globalThis.process.env.VANEHUB_DESKTOP_RESULT_DIR;
    if (resultDir) {
      const { stdout: diff } = await run("git", ["show", "--stat", "-p", "HEAD"], { cwd: target.dest });
      await writeFile(join(resultDir, "longrun-feature.patch"), diff);
      await writeFile(join(resultDir, "longrun-thread.json"), `${JSON.stringify({
        branch: target.branch,
        worktree: target.dest,
        thread: (await listMessages(flow.session.id)).map((row) => ({
          role: row.role,
          speakerSeatId: row.speakerSeatId ?? null,
          status: row.status ?? null,
          createdAt: row.createdAt ?? null,
          content: row.content,
        })),
      }, null, 2)}\n`);
    }
  });

  globalThis.after(async () => {
    if (flow.session) {
      await invoke(({ core }, id) => core.invoke("stop_generation", { sessionId: id }), flow.session.id)
        .catch(() => {});
      if (!flow.failed && globalThis.process?.env?.VANEHUB_DESKTOP_KEEP_SESSIONS !== "1") {
        await invoke(({ core }, id) => core.invoke("delete_session", { sessionId: id }), flow.session.id)
          .catch(() => {});
      }
    }
    if (target) {
      globalThis.console.warn(`Task worktree kept for review: ${target.dest} (branch ${target.branch})`);
    }
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
