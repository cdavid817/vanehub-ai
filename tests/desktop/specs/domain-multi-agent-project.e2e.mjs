import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { access, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];
const stamp = Date.now().toString(36);

/**
 * A multi-file feature moving through TWO relay rounds of a three-CLI group chat.
 *
 * `domain-multi-agent-business.e2e.mjs` proves one pass of 架构师 → 实现者 → 代码审查 over a
 * single file. What it cannot show is the shape real work actually has: several files, a review
 * that sends work BACK, and a seat taking a second turn on the same thread — the depth-3 chain
 * and the re-dispatch of a seat that has already spoken are runtime paths no other spec reaches.
 *
 * The correctness judge is `python3 -m unittest`, run by this harness against the repository
 * after the implementer's turns — an Agent saying "done" proves compliance, a passing test run
 * proves work.
 *
 * Seat order and trust policy follow the business spec's established findings verbatim: codex-cli
 * takes the words-only seat (its `workspace-write` sandbox cannot start under
 * `apparmor_restrict_unprivileged_userns=1`), and the acting seats run `trusted` because a seat
 * turn is headless and `standard` means ask-before-acting with nobody at the prompt.
 *
 * The seats are told to write files and never run commands, because of a third finding this file
 * itself produced: claude-code's `trusted` (and `yolo`) both project to `acceptEdits`, which
 * auto-approves edits but not shell commands — command approval belongs to the permission-hook
 * relay, which an isolated run deliberately does not install into the user's real `~/.claude`.
 * The first honest run had the implementer, correctly, halt the round with a line-leading
 * `@用户 handoff` when its self-test hit that gate — live proof of the "Blocking handoff pauses
 * the round" scenario, and of why the objective test run below belongs to the harness.
 */
const TRIO = ["codex-cli", "claude-code", "opencode"];
const TRUSTED_SEATS = ["claude-code", "opencode"];
const BUILTIN_ROLES = ["builtin-architect", "builtin-implementer", "builtin-reviewer"];
const PROJECT_FILES = ["inventory.py", "cli.py", join("tests", "test_inventory.py")];
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
  const root = await mkdtemp(join(tmpdir(), "vanehub-multiagent-project-"));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "README.md"), "# 库存管理\n\n一个待实现的最小库存管理库。\n", "utf8");
  await run("git", ["add", "README.md"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

/** The objective judge: the repository's own test suite, run outside every model's control. */
async function unitTestsPass(root) {
  try {
    const { stdout, stderr } = await run(
      "python3",
      ["-m", "unittest", "discover", "-s", "tests", "-v"],
      { cwd: root, timeout: 60_000 },
    );
    return { passed: true, output: `${stdout}\n${stderr}` };
  } catch (error) {
    return { passed: false, output: `${error.stdout ?? ""}\n${error.stderr ?? error.message}` };
  }
}

let repository = null;
const flow = { session: null, seats: [], handles: [], failed: false };

const listMessages = (sessionId) => invoke(({ core }, id) => core.invoke("list_messages", {
  sessionId: id,
  limit: null,
  beforeId: null,
}), sessionId);

const assistantsOf = (messages) => messages.filter((message) => message.role === "assistant");

/**
 * The `ordinal`-th assistant turn overall, required to belong to `seat`, dispatched and settled.
 * Ordinal-addressed rather than seat-addressed because the implementer speaks twice: "the seat's
 * row" stops identifying a turn as soon as any seat has more than one.
 */
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
    return null;
  }
  return settled;
}

function bail(context) {
  flow.failed = true;
  context.skip();
}

globalThis.describe("VaneHub AI desktop multi-Agent project development", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    repository = await createRepository();
  });

  globalThis.it("seats the trio and the 架构师 lays out the module structure", async function architect() {
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const lineupAgents = TRIO.map((id) => agents.find((agent) => agent.id === id
      && agent.availabilityState === "available"
      && agent.supportedInteractionModes.includes("cli")));
    const missing = TRIO.filter((_, index) => !lineupAgents[index]);
    if (missing.length > 0) {
      blocked.push(`project flow: ${missing.join(", ")} not installed/usable on this host`);
      bail(this);
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const seatRoles = BUILTIN_ROLES.map((id) => roles.find((role) => role.id === id)).filter(Boolean);
    if (seatRoles.length < 3) {
      blocked.push("project flow: fewer than three built-in expert roles are available");
      bail(this);
    }
    for (const agentId of TRUSTED_SEATS) {
      await invoke(({ core }, input) => core.invoke("apply_policy_template", { input }), {
        agentId,
        template: "trusted",
      });
    }

    const title = `multiagent-project-${stamp}`;
    const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
      agentId: lineupAgents[0].id,
      interactionMode: "cli",
      title,
      folder: repository,
      projectPath: repository,
      remoteWorkspace: null,
      worktree: null,
    });
    await settleOperation(operation, "Creating the project session never settled.");
    const created = await globalThis.browser.waitUntil(async () => {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === title) ?? false;
    }, { timeout: 30_000, timeoutMsg: "The project session was not created." });

    flow.session = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
      sessionId: created.id,
      expectedUpdatedAt: created.updatedAt,
      seats: lineupAgents.map((agent, index) => ({ agentId: agent.id, roleId: seatRoles[index].id })),
    });
    flow.seats = flow.session.seats.filter((seat) => !seat.leftAt);
    flow.handles = seatRoles.map((role) => role.displayName.split(/\s+/u).filter(Boolean).join("-"));
    assert.deepEqual(flow.seats.map((seat) => seat.agentId), TRIO);

    // The requirement, with each seat's marching orders and the mention discipline spelled out.
    // The review round-trip is mandated by the words, so the runtime's re-dispatch of an
    // already-spoken seat gets exercised regardless of how good the first implementation is.
    await invoke(({ core }, payload) => core.invoke("send_message", payload), {
      sessionId: flow.session.id,
      content: [
        `@${flow.handles[0]} 我们开发一个最小库存管理库，多文件项目，分工与顺序必须严格遵守：`,
        "",
        "1. 架构师（你）：只做设计，不写实现。给出三个文件的结构与接口约定：",
        "   - inventory.py：class Inventory，方法 add_item(name, quantity)、",
        "     remove_item(name, quantity)、stock_of(name)、low_stock(threshold)；",
        "     数量必须为正整数，移除超过库存或未知条目要抛 ValueError/KeyError。",
        "   - cli.py：main(argv) 纯函数式入口，支持 add/remove/stock 三个子命令，返回退出码。",
        "   - tests/test_inventory.py：unittest 覆盖正常路径与异常路径。",
        `   设计写完后，最后单独起一行、只写 @${flow.handles[1]}。`,
        "2. 实现者：你拥有本仓库文件写权限，但没有命令执行权限。按设计创建全部三个文件并实现",
        "   （含 docstring），不要征求确认，也不要尝试运行任何命令——测试由外部系统执行。",
        `   文件写完后，最后单独起一行、只写 @${flow.handles[2]}。`,
        "3. 代码审查：读取全部三个文件，必须提出恰好一条需要实现者修改的具体意见",
        "   （引用文件与行内内容，例如补一处遗漏的校验或边界测试），不要运行命令，",
        `   然后最后单独起一行、只写 @${flow.handles[1]}，把工作交回实现者。`,
        "4. 实现者（第二轮）：按审查意见直接修改文件，不要运行命令，",
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

    const turn = await turnAt("architect", 0, flow.seats[0], flow.handles[0]);
    if (!turn) bail(this);
  });

  globalThis.it("the 实现者 builds the three-file project and its tests pass", async function implement() {
    if (flow.failed) this.skip();
    const turn = await turnAt("implementer", 1, flow.seats[1], flow.handles[1]);
    if (!turn) bail(this);

    for (const file of PROJECT_FILES) {
      await access(join(repository, file)).catch(() => {
        flow.failed = true;
        assert.fail(`${file} was not created; the implementer's completed turn said: `
          + JSON.stringify(turn.content.slice(0, 300)));
      });
    }
    // The judge is the project's own suite run by the harness, not the Agent's claim of success.
    const verdict = await unitTestsPass(repository);
    assert.ok(verdict.passed, `the implemented project fails its own tests:\n${verdict.output.slice(0, 1_500)}`);
  });

  globalThis.it("the 代码审查 sends concrete rework back to the 实现者", async function review() {
    if (flow.failed) this.skip();
    const turn = await turnAt("reviewer", 2, flow.seats[2], flow.handles[2]);
    if (!turn) bail(this);
    assert.ok(
      turn.content.includes("inventory") || turn.content.includes("cli")
        || turn.content.includes("test"),
      `the review does not reference the project's files (got ${JSON.stringify(turn.content.slice(0, 160))})`,
    );
  });

  globalThis.it("the 实现者 takes a second turn on the same thread and the round closes", async function rework() {
    if (flow.failed) this.skip();
    // The runtime path under test: a seat that has already spoken is dispatched again, at chain
    // depth 3, in the same round. Ordinal addressing is what distinguishes this turn from its
    // first one.
    const turn = await turnAt("rework", 3, flow.seats[1], flow.handles[1]);
    if (!turn) bail(this);

    const verdict = await unitTestsPass(repository);
    assert.ok(verdict.passed, `after rework the project fails its own tests:\n${verdict.output.slice(0, 1_500)}`);

    // Attribution across rounds: the mandated relay is the thread's PREFIX, not its entirety.
    // The Agents own the tail — a first honest run had the implementer hand its rework back with
    // `@代码审查 复核` and the reviewer take a fifth turn, which is the collaboration working,
    // not the test failing. What stays asserted is that every turn, however many the seats chose
    // to take, belongs to a known seat.
    const rows = assistantsOf(await listMessages(flow.session.id));
    const speakers = rows.map((row) => row.speakerSeatId);
    assert.deepEqual(
      speakers.slice(0, 4),
      [flow.seats[0], flow.seats[1], flow.seats[2], flow.seats[1]].map((seat) => seat.seatId),
      "the thread does not begin with the relay 架构师 → 实现者 → 代码审查 → 实现者",
    );
    const known = new Set(flow.seats.map((seat) => seat.seatId));
    assert.ok(
      speakers.every((seatId) => known.has(seatId)),
      `a turn in the thread belongs to no seated participant: ${JSON.stringify(speakers)}`,
    );
    // However long the tail ran, the round has to have ended rather than ping-pong forever —
    // the seats stop being dispatched once a turn names nobody (or hands off to the human),
    // and MAX_CHAIN_DEPTH bounds the pathological case.
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
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
