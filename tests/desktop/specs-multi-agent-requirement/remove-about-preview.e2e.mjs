import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
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
 * A REAL product requirement carried end to end by a two-seat group chat: the codex-cli 架构师
 * designs, the claude-code 实现者 edits, and the change lands in a disposable git worktree of
 * THIS repository — "settings → about: remove the Preview badge".
 *
 * What distinguishes this from `domain-multi-agent-project.e2e.mjs` is the workspace: not a
 * synthetic fixture repo, but the product's own source tree, so the implementer has to locate
 * `about-page.tsx` / `about-service.ts` among thousands of files from the requirement text alone.
 *
 * Host findings inherited from the business/project specs: codex-cli takes the words-only seat
 * (its sandbox cannot start under `apparmor_restrict_unprivileged_userns=1`, and an architect
 * does not need write access anyway); claude-code runs the `trusted` template (projects to
 * acceptEdits — file writes auto-approved, commands not), so the seats are told to write files
 * and never run commands. Lint and review are the harness's job, done after the round.
 *
 * The judge is mechanical and spelled out verbatim in the requirement message, so a completed
 * round that fails it is the collaboration failing, not the test guessing at intent.
 */
const DUO = ["codex-cli", "claude-code"];
const ROLES = ["builtin-architect", "builtin-implementer"];
const STAGE_BUDGET_MS = 6 * 60 * 1000;

const specDir = path.dirname(fileURLToPath(import.meta.url));
// tests/desktop/specs-multi-agent-requirement → the checkout that owns this spec.
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

/** A throwaway worktree of this repository, branched off main, for the seats to edit. */
async function createTargetWorktree() {
  const dest = join(await mkdtemp(join(tmpdir(), "vanehub-agent-task-")), "repo");
  const branch = `agents/remove-about-preview-${stamp}`;
  await run("git", ["worktree", "add", dest, "-b", branch, "main"], { cwd: repoRoot });
  return { dest, branch };
}

let target = null;
const flow = { session: null, seats: [], handles: [], failed: false };

const listMessages = (sessionId) => invoke(({ core }, id) => core.invoke("list_messages", {
  sessionId: id,
  limit: null,
  beforeId: null,
}), sessionId);

const assistantsOf = (messages) => messages.filter((message) => message.role === "assistant");

/** Everything the runtime knows about the session, written beside the wdio evidence. */
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
    join(resultDir, `diagnostics-${tag}.json`),
    `${JSON.stringify(capture, null, 2)}\n`,
  ).catch(() => {});
}

async function turnAt(stage, ordinal, seat, handle) {
  const row = await globalThis.browser.waitUntil(async () => {
    const rows = assistantsOf(await listMessages(flow.session.id));
    return rows.length > ordinal ? rows[ordinal] : false;
  }, {
    timeout: STAGE_BUDGET_MS,
    interval: 3_000,
    timeoutMsg: `turn ${ordinal} (${handle}) was never dispatched`,
  }).catch(() => null);
  if (!row) {
    const rows = assistantsOf(await listMessages(flow.session.id));
    const previous = rows[ordinal - 1];
    if (previous && !(previous.content ?? "").includes(`@${handle}`)) {
      blocked.push(`${stage}: the previous turn did not emit @${handle}, so the relay had `
        + `nothing to act on (it replied: ${JSON.stringify((previous.content ?? "").slice(0, 160))})`);
      return null;
    }
    flow.failed = true;
    assert.fail(`turn ${ordinal} for @${handle} (${seat.agentId}) was never dispatched`);
  }
  assert.equal(row.speakerSeatId, seat.seatId, `turn ${ordinal} belongs to a seat other than @${handle}`);
  const settled = await globalThis.browser.waitUntil(async () => {
    const current = assistantsOf(await listMessages(flow.session.id))[ordinal];
    return ["completed", "failed", "cancelled"].includes(current?.status) ? current : false;
  }, {
    timeout: STAGE_BUDGET_MS,
    interval: 3_000,
    timeoutMsg: `turn ${ordinal} (${handle}) never settled`,
  }).catch(() => null);
  if (!settled || settled.status !== "completed" || !settled.content.trim()) {
    blocked.push(`${stage}: the ${seat.agentId} turn ended ${settled?.status ?? "never"} `
      + `(${JSON.stringify((settled?.error ?? settled?.content ?? "").slice(0, 160))}); that `
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

/** The mechanical acceptance criteria the requirement message promises the seats. */
async function judge() {
  const failures = [];
  const { stdout: porcelain } = await run("git", ["status", "--porcelain"], { cwd: target.dest });
  if (!porcelain.trim()) failures.push("the worktree has no changes at all");
  if (!porcelain.includes("src/settings/pages/about-page.tsx")) {
    failures.push("src/settings/pages/about-page.tsx was not modified");
  }
  const aboutPage = await readFile(join(target.dest, "src/settings/pages/about-page.tsx"), "utf8");
  if (aboutPage.includes("aboutBuildChannel")) failures.push("about-page.tsx still references aboutBuildChannel");
  if (aboutPage.includes("Preview")) failures.push('about-page.tsx still contains a hardcoded "Preview"');
  const aboutService = await readFile(join(target.dest, "src/services/about-service.ts"), "utf8");
  if (aboutService.includes('"Preview"')) failures.push('about-service.ts still contains "Preview"');
  return failures;
}

globalThis.describe("VaneHub AI multi-Agent delivers a real product requirement", () => {
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

  globalThis.it("seats the duo and the 架构师 designs the removal", async function architect() {
    this.timeout(STAGE_BUDGET_MS * 2 + 180_000);
    // Wait out the startup detection refresh: creating the session while probes are still
    // running raced the runtime on the first honest run of this file.
    const lineupAgents = await globalThis.browser.waitUntil(async () => {
      // A transient list_agents failure during the startup refresh must read as "not yet",
      // not abort the whole wait — waitUntil rethrows a throwing condition immediately.
      const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }))
        .catch(() => null);
      if (!Array.isArray(agents)) return false;
      const lineup = DUO.map((id) => agents.find((agent) => agent.id === id
        && agent.availabilityState === "available"
        && agent.supportedInteractionModes.includes("cli")));
      return lineup.every(Boolean) ? lineup : false;
    }, { timeout: 120_000, interval: 3_000, timeoutMsg: "codex-cli/claude-code never became available" })
      .catch(() => null);
    if (!lineupAgents) {
      const resultDir = globalThis.process.env.VANEHUB_DESKTOP_RESULT_DIR;
      const snapshot = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }))
        .catch((error) => String(error));
      if (resultDir) {
        await writeFile(join(resultDir, "diagnostics-list-agents.json"), `${JSON.stringify(
          Array.isArray(snapshot)
            ? snapshot.filter((agent) => DUO.includes(agent.id))
            : snapshot,
          null,
          2,
        )}\n`).catch(() => {});
      }
      blocked.push(`requirement flow: ${DUO.join(", ")} not both installed/usable on this host`);
      bail(this);
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const seatRoles = ROLES.map((id) => roles.find((role) => role.id === id)).filter(Boolean);
    if (seatRoles.length < 2) {
      blocked.push("requirement flow: built-in architect/implementer roles are unavailable");
      bail(this);
    }
    await invoke(({ core }, input) => core.invoke("apply_policy_template", { input }), {
      agentId: "claude-code",
      template: "trusted",
    });

    const title = `multiagent-requirement-${stamp}`;
    const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
      agentId: lineupAgents[0].id,
      interactionMode: "cli",
      title,
      folder: target.dest,
      projectPath: target.dest,
      remoteWorkspace: null,
      worktree: null,
    });
    await settleOperation(operation, "Creating the requirement session never settled.");
    const created = await globalThis.browser.waitUntil(async () => {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === title) ?? false;
    }, { timeout: 30_000, timeoutMsg: "The requirement session was not created." });

    flow.session = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
      sessionId: created.id,
      expectedUpdatedAt: created.updatedAt,
      seats: lineupAgents.map((agent, index) => ({ agentId: agent.id, roleId: seatRoles[index].id })),
    });
    flow.seats = flow.session.seats.filter((seat) => !seat.leftAt);
    flow.handles = seatRoles.map((role) => role.displayName.split(/\s+/u).filter(Boolean).join("-"));
    assert.deepEqual(flow.seats.map((seat) => seat.agentId), DUO);

    // The real requirement, with the judge's criteria quoted verbatim so a completed round that
    // fails them is a collaboration failure, not a moving goalpost.
    await invoke(({ core }, payload) => core.invoke("send_message", payload), {
      sessionId: flow.session.id,
      content: [
        `@${flow.handles[0]} 真实产品需求:当前仓库是 VaneHub AI 桌面应用的源码。`,
        "「设置 → 关于」页面标题旁有一个绿色的 \"Preview\" 徽标,产品决定将它移除。分工与纪律:",
        "",
        "1. 架构师(你):只做设计,不改代码。先定位相关源文件,给出改动方案:要删/改哪些代码、",
        "   因此不再被使用的常量或文案如何清理、「发布通道」信息行如何处理,并说明取舍。",
        `   方案写完后,最后单独起一行、只写 @${flow.handles[1]}。`,
        "2. 实现者:你拥有仓库文件写权限,但没有命令执行权限。按方案直接修改文件,不要征求确认,",
        "   也不要运行任何命令——lint 与测试由外部系统执行。外部系统的验收标准(逐字执行):",
        "   a) src/settings/pages/about-page.tsx 中不再出现 aboutBuildChannel,也不再出现 Preview 字样;",
        "   b) src/services/about-service.ts 中不再出现 \"Preview\" 字符串;",
        "   c) 不留下未使用的导入或导出常量。",
        `   改完说明动了哪些文件与原因,然后最后单独起一行、只写 @用户 done。`,
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

    const turn = await turnAt("architect", 0, flow.seats[0], flow.handles[0]);
    if (!turn) bail(this);
    // A design that never names the badge's home has not located the work; the implementer would
    // be starting the search over, which is the architect seat failing its half of the split.
    assert.ok(
      /about/iu.test(turn.content),
      `the design never mentions the about page (got ${JSON.stringify(turn.content.slice(0, 200))})`,
    );
  });

  globalThis.it("the 实现者 lands the change and the acceptance criteria hold", async function implement() {
    this.timeout(STAGE_BUDGET_MS * 2 + 120_000);
    if (flow.failed) this.skip();
    const turn = await turnAt("implementer", 1, flow.seats[1], flow.handles[1]);
    if (!turn) bail(this);

    const failures = await judge();
    assert.deepEqual(failures, [], `acceptance criteria failed:\n  ${failures.join("\n  ")}\n`
      + `implementer's completed turn said: ${JSON.stringify(turn.content.slice(0, 400))}`);
  });

  globalThis.it("the round closes and the harness records the evidence", async function close() {
    this.timeout(STAGE_BUDGET_MS + 120_000);
    if (flow.failed) this.skip();
    await globalThis.browser.waitUntil(async () => {
      const current = assistantsOf(await listMessages(flow.session.id));
      return current.every((row) => ["completed", "failed", "cancelled"].includes(row.status))
        ? current.length
        : false;
    }, {
      timeout: STAGE_BUDGET_MS,
      interval: 3_000,
      timeoutMsg: "the round never settled: some turn is still streaming",
    });
    const rows = assistantsOf(await listMessages(flow.session.id));
    const known = new Set(flow.seats.map((seat) => seat.seatId));
    assert.ok(
      rows.every((row) => known.has(row.speakerSeatId)),
      "a turn in the thread belongs to no seated participant",
    );

    // Commit on the task branch so the run's outcome survives worktree cleanup and can be
    // reviewed or cherry-picked by a human afterwards.
    await run("git", ["add", "-A"], { cwd: target.dest });
    await run("git", [
      "-c", "user.name=VaneHub Multi-Agent E2E",
      "-c", "user.email=desktop-e2e@example.invalid",
      "commit", "-m", "feat: remove Preview badge from Settings About page\n\n"
        + "Produced by a VaneHub AI multi-agent session: codex-cli architect designed,\n"
        + "claude-code implementer edited; acceptance judged mechanically by the harness.",
    ], { cwd: target.dest });

    const resultDir = globalThis.process.env.VANEHUB_DESKTOP_RESULT_DIR;
    if (resultDir) {
      const { stdout: diff } = await run("git", ["show", "--stat", "-p", "HEAD"], { cwd: target.dest });
      await writeFile(join(resultDir, "multi-agent-requirement.patch"), diff);
      await writeFile(join(resultDir, "multi-agent-thread.json"), `${JSON.stringify({
        branch: target.branch,
        worktree: target.dest,
        thread: (await listMessages(flow.session.id)).map((row) => ({
          role: row.role,
          speakerSeatId: row.speakerSeatId ?? null,
          status: row.status ?? null,
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
