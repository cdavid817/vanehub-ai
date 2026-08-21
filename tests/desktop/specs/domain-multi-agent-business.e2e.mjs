import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { access, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];
const stamp = Date.now().toString(36);

/**
 * One real piece of work flowing through a three-CLI group chat.
 *
 * The routing suite proves the plumbing: mentions dispatch, seats answer, chains relay. None of it
 * proves the *point* of a group chat — that three heterogeneous CLI Agents seated as 架构师 →
 * 实现者 → 代码审查 can move an actual coding task through the thread, each doing its part in the
 * shared repository. This spec runs that: a concrete feature request goes to the architect, the
 * implementer writes the file into the fixture repo, the reviewer reads what was written.
 *
 * Shaped as three sequential steps rather than one case, for two reasons learned the hard way:
 * the harness's per-test ceiling is five minutes and `this.timeout()` does not raise it under the
 * WDIO mocha wrapper, so one case spanning three working provider round trips gets killed
 * mid-flight; and a step that fails must leave the session's thread in the database — the first
 * failing run deleted its own evidence in `after`, which turned "codex completed without writing
 * the file" into an unanswerable why.
 *
 * Asserted and reported at different strengths, deliberately:
 * - Dispatch, attribution, and per-turn completion are ASSERTED — they are this product's code.
 * - Each provider following its handoff instruction (`@下一位` at line start) is REPORTED as
 *   BLOCKED when declined — the suite cannot hold a model to an instruction, only observe it.
 * - The implementer's file landing in the repo is ASSERTED once its turn completed, with the
 *   turn's own words printed on failure so a refusal is distinguishable from a runtime fault.
 *
 * Seat order and policy are chosen from what this file's first honest runs established about
 * working headlessly, not from preference:
 * - codex-cli sits as 架构师, the one seat whose work is words. Its `workspace-write` sandbox
 *   cannot start on a host with `kernel.apparmor_restrict_unprivileged_userns=1` (Ubuntu 24.04+
 *   default): bwrap fails with `loopback: Failed RTM_NEWADDR: Operation not permitted`, verified
 *   identically outside this app — and both `standard` and `trusted` map codex to
 *   `workspace-write`, so no template the product will assign lets codex write here. That is a
 *   real product gap (silent, no diagnostic) documented by this spec, not worked around silently.
 * - The seats that must *act* (write, then read) get the `trusted` template through the product's
 *   own `apply_policy_template`, because `standard` means ask-before-acting and a seat turn is
 *   headless — there is nobody at the approval prompt. Verified: claude-code under
 *   `permissionMode=default` declines headless writes and says to use acceptEdits.
 */
const TRIO = ["codex-cli", "claude-code", "opencode"];
const TRUSTED_SEATS = ["claude-code", "opencode"];
const BUILTIN_ROLES = ["builtin-architect", "builtin-implementer", "builtin-reviewer"];
const FEATURE_FILE = "temperature.py";
const STAGE_BUDGET_MS = 4 * 60 * 1000;

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

async function createRepository() {
  const root = await mkdtemp(join(tmpdir(), "vanehub-multiagent-business-"));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "README.md"), "# 温度工具\n\n一个待补全的温度换算小工具。\n", "utf8");
  await run("git", ["add", "README.md"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

let repository = null;
/** Shared across the steps; each step hard-skips when its predecessor did not hand over. */
const flow = { session: null, seats: [], handles: [], turns: [], failed: false };

const listMessages = (sessionId) => invoke(({ core }, id) => core.invoke("list_messages", {
  sessionId: id,
  limit: null,
  beforeId: null,
}), sessionId);

const assistantsOf = (messages) => messages.filter((message) => message.role === "assistant");

/** The named seat's turn, dispatched and settled, or null with the reason pushed to `blocked`. */
async function seatTurn(stage) {
  const index = flow.turns.length;
  const seat = flow.seats[index];
  const handle = flow.handles[index];
  const row = await globalThis.browser.waitUntil(async () => {
    const rows = assistantsOf(await listMessages(flow.session.id));
    return rows.find((candidate) => candidate.speakerSeatId === seat.seatId) ?? false;
  }, {
    timeout: STAGE_BUDGET_MS,
    interval: 3_000,
    timeoutMsg: `the ${handle} seat was never dispatched`,
  }).catch(() => null);
  if (!row) {
    const previous = flow.turns[index - 1];
    if (previous && !previous.content.includes(`@${handle}`)) {
      blocked.push(`${stage}: ${flow.seats[index - 1].agentId} did not emit @${handle}, so the `
        + `relay had nothing to act on (replied: ${JSON.stringify(previous.content.slice(0, 160))})`);
      return null;
    }
    flow.failed = true;
    assert.fail(`the ${handle} seat (${seat.agentId}) was never dispatched`);
  }
  const settled = await globalThis.browser.waitUntil(async () => {
    const rows = assistantsOf(await listMessages(flow.session.id));
    const current = rows.find((candidate) => candidate.speakerSeatId === seat.seatId);
    return ["completed", "failed", "cancelled"].includes(current?.status) ? current : false;
  }, {
    timeout: STAGE_BUDGET_MS,
    interval: 3_000,
    timeoutMsg: `the ${handle} turn never settled`,
  }).catch(() => null);
  if (!settled || settled.status !== "completed" || !settled.content.trim()) {
    blocked.push(`${stage}: the ${seat.agentId} turn ended ${settled?.status ?? "never"} `
      + `(${JSON.stringify((settled?.error ?? settled?.content ?? "").slice(0, 160))}); that `
      + "seat's runtime failed, not the relay");
    return null;
  }
  flow.turns.push(settled);
  return settled;
}

/** Marks the flow dead and skips, so later steps report "predecessor" instead of re-failing. */
function bail(context) {
  flow.failed = true;
  context.skip();
}

globalThis.describe("VaneHub AI desktop multi-Agent business flow", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    repository = await createRepository();
  });

  globalThis.it("seats the trio and the 架构师 turns the request into a design", async function architect() {
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const lineupAgents = TRIO.map((id) => agents.find((agent) => agent.id === id
      && agent.availabilityState === "available"
      && agent.supportedInteractionModes.includes("cli")));
    const missing = TRIO.filter((_, index) => !lineupAgents[index]);
    if (missing.length > 0) {
      blocked.push(`business flow: ${missing.join(", ")} not installed/usable on this host`);
      bail(this);
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const seatRoles = BUILTIN_ROLES.map((id) => roles.find((role) => role.id === id)).filter(Boolean);
    if (seatRoles.length < 3) {
      blocked.push("business flow: fewer than three built-in expert roles are available");
      bail(this);
    }

    // The acting seats need launch-time permission to act; the fresh per-run data directory means
    // this assignment lives and dies with the test.
    for (const agentId of TRUSTED_SEATS) {
      await invoke(({ core }, input) => core.invoke("apply_policy_template", { input }), {
        agentId,
        template: "trusted",
      });
    }

    const title = `multiagent-business-${stamp}`;
    const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
      agentId: lineupAgents[0].id,
      interactionMode: "cli",
      title,
      folder: repository,
      projectPath: repository,
      remoteWorkspace: null,
      worktree: null,
    });
    await settleOperation(operation, "Creating the business session never settled.");
    const created = await globalThis.browser.waitUntil(async () => {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === title) ?? false;
    }, { timeout: 30_000, timeoutMsg: "The business session was not created." });

    flow.session = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
      sessionId: created.id,
      expectedUpdatedAt: created.updatedAt,
      seats: lineupAgents.map((agent, index) => ({ agentId: agent.id, roleId: seatRoles[index].id })),
    });
    flow.seats = flow.session.seats.filter((seat) => !seat.leftAt);
    flow.handles = seatRoles.map((role) => role.displayName.split(/\s+/u).filter(Boolean).join("-"));
    assert.deepEqual(flow.seats.map((seat) => seat.agentId), TRIO);

    // A concrete, checkable task. Each seat's marching orders are in the message itself so the
    // test measures the relay and the work, not each model's initiative; the mention discipline
    // (`@下一位` alone at line start) mirrors what the runtime's own briefing asks of them.
    await invoke(({ core }, payload) => core.invoke("send_message", payload), {
      sessionId: flow.session.id,
      content: [
        `@${flow.handles[0]} 我们来完成一个小功能，分工如下，请严格遵守：`,
        "",
        `1. 架构师（你）：为本仓库设计 ${FEATURE_FILE}，包含 celsius_to_fahrenheit(c) 与`,
        "   fahrenheit_to_celsius(f) 两个函数的接口约定（参数、返回、异常），不要写实现代码。",
        `   写完设计后，最后单独起一行、只写 @${flow.handles[1]} 三个字之外不带任何前缀，交给实现者。`,
        `2. 实现者：你拥有本仓库的写权限。按设计立即在仓库根目录创建 ${FEATURE_FILE} 并实现`,
        "   两个函数（含 docstring）。不要征求确认、不要只贴代码——必须真实写入文件。",
        `   写入完成后，最后单独起一行、只写 @${flow.handles[2]}，交给代码审查。`,
        `3. 代码审查：读取仓库里的 ${FEATURE_FILE}，指出至少一条具体改进意见（引用行内内容），`,
        "   然后最后单独起一行、只写 @用户 done，结束本轮。",
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

    const turn = await seatTurn("architect");
    if (!turn) bail(this);
  });

  globalThis.it("the 实现者 writes the designed module into the shared repository", async function implementer() {
    if (flow.failed) this.skip();
    const turn = await seatTurn("implementer");
    if (!turn) bail(this);

    // The work product is on disk in the session's repository. The turn completed, so an absent
    // file means either the runtime ran the CLI somewhere it could not do the work, or the model
    // answered instead of acting — the turn's own words, printed here, say which.
    await access(join(repository, FEATURE_FILE)).catch(() => {
      assert.fail(`${FEATURE_FILE} was not created in the session repository; the implementer's `
        + `completed turn said: ${JSON.stringify(turn.content.slice(0, 400))}`);
    });
    const source = await readFile(join(repository, FEATURE_FILE), "utf8");
    assert.ok(
      source.includes("celsius_to_fahrenheit") && source.includes("fahrenheit_to_celsius"),
      `${FEATURE_FILE} exists but does not implement the two requested functions`,
    );
  });

  globalThis.it("the 代码审查 reads the file and closes the round attributably", async function reviewer() {
    if (flow.failed) this.skip();
    const turn = await seatTurn("reviewer");
    if (!turn) bail(this);

    // The reviewer engaged with the artefact rather than answering from the air.
    assert.ok(
      turn.content.includes("temperature")
        || turn.content.includes("celsius")
        || turn.content.includes("fahrenheit"),
      `the review does not reference the implemented file (got ${JSON.stringify(turn.content.slice(0, 160))})`,
    );

    // Every turn in the round belongs to a known seat, in relay order — the shared thread stayed
    // attributable end to end.
    const rows = assistantsOf(await listMessages(flow.session.id));
    assert.deepEqual(
      rows.map((row) => row.speakerSeatId),
      flow.seats.map((seat) => seat.seatId),
      "the thread's turns are not the three seats speaking once each, in relay order",
    );
  });

  globalThis.after(async () => {
    if (flow.session) {
      await invoke(({ core }, id) => core.invoke("stop_generation", { sessionId: id }), flow.session.id)
        .catch(() => {});
      // A failed flow keeps its session: the thread in the run's database is the evidence for why
      // it failed, and the first version of this file deleted exactly that.
      if (!flow.failed && globalThis.process?.env?.VANEHUB_DESKTOP_KEEP_SESSIONS !== "1") {
        await invoke(({ core }, id) => core.invoke("delete_session", { sessionId: id }), flow.session.id)
          .catch(() => {});
      }
    }
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
