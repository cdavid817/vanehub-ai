import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import process from "node:process";
import { promisify } from "node:util";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

const shots = join(process.env.VANEHUB_DESKTOP_RESULT_DIR ?? tmpdir(), "loop-ui");
const fixtureRoot = process.env.VANEHUB_APP_DATA_DIR
  ? join(dirname(process.env.VANEHUB_APP_DATA_DIR), "fixtures")
  : tmpdir();
const stamp = Date.now().toString(36);
const definitionName = `ui-loop-${stamp}`;
let repository = null;

/**
 * Drives Loop Engineering the way a user reaches it: the /workspace/loops screen, the empty state,
 * the four-step definition wizard, the definition overview and the preflight dialog. The IPC-level
 * behaviour (validation, run-control guards) is covered by specs/domain-loop.e2e.mjs; what only
 * this spec covers is that the wizard's own controls produce a definition the backend accepts and
 * that preflight is reachable and readable without starting a run.
 *
 * Selectors are written against zh-CN and the language is pinned first (tests/desktop/helpers/
 * native-ui.mjs explains why). Preflight check *presence* is asserted, not passing status: whether
 * worker/verifier agents are eligible depends on which CLIs the host has.
 */
async function capture(name) {
  await globalThis.browser.saveScreenshot(join(shots, `${name}.png`));
}

async function navigate(path) {
  await globalThis.browser.execute((target) => {
    globalThis.history.pushState({}, "", target);
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  }, path);
}

const dialog = () => globalThis.$('[role="dialog"]');
const dialogButton = (label) => globalThis.$(`//*[@role="dialog"]//button[normalize-space(.)="${label}"]`);
const fieldControl = (label, control) =>
  globalThis.$(`//*[@role="dialog"]//label[.//span[normalize-space(text())="${label}"]]//${control}`);

async function setField(label, control, value) {
  const element = await fieldControl(label, control);
  await element.waitForEnabled({ timeout: 20_000 });
  await element.setValue(value);
}

/**
 * WebKitGTK's WebDriver accepts `selectByAttribute` against a React-controlled <select> without
 * ever firing the change event React listens to, so the draft silently keeps its old value. Set
 * the value through the native setter and dispatch `change` ourselves. `wanted` starting with "*"
 * matches by suffix (project paths get canonicalised by the backend).
 */
async function selectByLabel(label, wanted) {
  const outcome = await globalThis.browser.execute((labelText, target) => {
    const labels = [...globalThis.document.querySelectorAll('[role="dialog"] label')];
    const select = labels
      .find((node) => node.querySelector("span")?.textContent.trim() === labelText)
      ?.querySelector("select");
    if (!select) return "no-select";
    const option = [...select.options].find((item) => target.startsWith("*")
      ? item.value.endsWith(target.slice(1))
      : item.value === target);
    if (!option) return `no-option among [${[...select.options].map((item) => item.value).join(", ")}]`;
    const setter = Object.getOwnPropertyDescriptor(globalThis.HTMLSelectElement.prototype, "value").set;
    setter.call(select, option.value);
    select.dispatchEvent(new globalThis.Event("change", { bubbles: true }));
    return "ok";
  }, label, wanted);
  assert.equal(outcome, "ok", `selecting ${wanted} for ${label} failed: ${outcome}`);
}

async function createRepository() {
  await mkdir(fixtureRoot, { recursive: true });
  const root = await mkdtemp(join(fixtureRoot, "loop-ui-"));
  await run("git", ["init", "-b", "main"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "seed.txt"), "seed\n", "utf8");
  await run("git", ["add", "seed.txt"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

/**
 * The wizard's project dropdown lists known projects (list_known_projects), and a project becomes
 * known by having hosted a session. Register the fixture repository the same way a user would
 * have: create one session in it.
 */
async function registerProject(root) {
  const operation = await invoke(({ core }, payload) => core.invoke("create_session", { input: payload }), {
    agentId: "claude-code",
    interactionMode: "cli",
    title: `loop-ui-registrar-${stamp}`,
    remoteWorkspace: null,
    worktree: null,
    folder: root,
    projectPath: root,
  });
  const settled = await globalThis.browser.waitUntil(async () => {
    const status = await invoke(
      ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
      operation.id,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
  }, { timeout: 60_000, timeoutMsg: "Registering the fixture project never settled." });
  assert.equal(settled.status, "succeeded", settled.error ?? "session creation failed");
  const known = await invoke(({ core }) => core.invoke("list_known_projects"));
  assert.ok(
    known.some((project) => basename(project.path) === basename(root)),
    "the fixture repository did not become a known project",
  );
}

globalThis.describe("VaneHub AI desktop Loop Engineering UI", () => {
  globalThis.before(async () => {
    await mkdir(shots, { recursive: true });
    await globalThis.browser.refresh();
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    await invoke(({ core }) => core.invoke("save_setting", {
      input: { key: "applicationLanguage", value: "zh-CN" },
    }));
    repository = await createRepository();
    await registerProject(repository);
  });

  globalThis.it("shows the Loop centre empty state with a create action", async () => {
    await navigate("/workspace/runs/loops");
    const frame = await globalThis.$('[data-testid="workspace-frame"]');
    await frame.waitForExist({ timeout: 20_000 });
    const create = await globalThis.$('//*[@role="main"]//button[normalize-space(.)="创建循环定义"]');
    await create.waitForClickable({ timeout: 20_000 });
    const text = await (await globalThis.$('[role="main"]')).getText();
    assert.ok(text.includes("暂无循环定义"), "the empty state headline is missing");
    assert.ok(text.includes("循环定义描述一段可重复执行的任务"), "the empty state explanation is missing");
    await capture("01-empty-state");
  });

  globalThis.it("refuses to advance past an empty scope step and says why", async () => {
    const create = await globalThis.$('//*[@role="main"]//button[normalize-space(.)="创建循环定义"]');
    await create.click();
    await (await dialog()).waitForExist({ timeout: 20_000 });
    await capture("02-wizard-scope-blank");

    const next = await dialogButton("下一步");
    await next.click();
    const footer = await globalThis.$('//*[@role="dialog"]//footer//p');
    await globalThis.browser.waitUntil(
      async () => (await footer.getText()).includes("必填项"),
      { timeout: 10_000, timeoutMsg: "The scope validation message never appeared." },
    );
    await capture("03-wizard-scope-error");
  });

  globalThis.it("walks the four-step wizard and saves a definition the backend accepts", async () => {
    await setField("名称", "input", definitionName);

    // The project options load from list_known_projects; wait for the fixture repo to be offered.
    const project = await fieldControl("项目路径", "select");
    await globalThis.browser.waitUntil(async () => {
      const values = await project.$$("option").map((option) => option.getAttribute("value"));
      return values.some((value) => value && basename(value) === basename(repository));
    }, { timeout: 20_000, timeoutMsg: "The fixture project never appeared in the project dropdown." });
    await selectByLabel("项目路径", `*${basename(repository)}`);

    const branch = await fieldControl("基础分支", "select");
    await globalThis.browser.waitUntil(async () => {
      const values = await branch.$$("option").map((option) => option.getAttribute("value"));
      return values.includes("main");
    }, { timeout: 20_000, timeoutMsg: "The main branch never appeared in the branch dropdown." });
    await selectByLabel("基础分支", "main");

    await setField("目标", "textarea", "保持 fixture 仓库处于绿色状态。");
    await setField("验收标准（每行一项）", "textarea", "seed.txt 仍然存在");
    await setField("允许路径（每行一项）", "textarea", "seed.txt");
    await capture("04-wizard-scope-filled");
    await (await dialogButton("下一步")).click();

    // Step 2: the wizard preselects the first registry entries, which need not be installed on
    // this host (the backend refuses to save a definition whose agent is unavailable). Choose the
    // two CLIs the host actually has, as a user would.
    const worker = await fieldControl("执行智能体", "select");
    await worker.waitForExist({ timeout: 10_000 });
    assert.notEqual(await worker.getValue(), "", "no worker agent was preselected");
    // The wizard must not preselect an agent the backend would refuse to save (this host has
    // available CLIs, so the default has to be one of them — unavailable options carry a suffix).
    const preselected = await globalThis.browser.execute(() => {
      const select = [...globalThis.document.querySelectorAll('[role="dialog"] label')]
        .find((node) => node.querySelector("span")?.textContent.trim() === "执行智能体")
        ?.querySelector("select");
      return select?.selectedOptions[0]?.textContent ?? "";
    });
    assert.ok(
      !/不可用|需要登录|状态未知/.test(preselected),
      `the wizard preselected an agent the backend would refuse: ${preselected}`,
    );
    await selectByLabel("执行智能体", "codex-cli");
    assert.equal(await worker.getValue(), "codex-cli", "the worker selection did not reach the draft");
    const verifier = await fieldControl("验证智能体", "select");
    assert.notEqual(await verifier.getValue(), "", "no verifier agent was preselected");
    await selectByLabel("验证智能体", "claude-code");
    assert.equal(await verifier.getValue(), "claude-code", "the verifier selection did not reach the draft");
    await capture("05-wizard-agents");
    await (await dialogButton("下一步")).click();

    // Step 3: a default verification command (npm run test) ships with the draft; make it cheap.
    const program = await fieldControl("验证程序", "input");
    await program.waitForExist({ timeout: 10_000 });
    await program.setValue("git");
    await setField("参数（每行一项）", "textarea", "status\n--porcelain");
    await capture("06-wizard-verification");
    await (await dialogButton("下一步")).click();

    const review = await globalThis.$('//*[@role="dialog"]//dl');
    await review.waitForExist({ timeout: 10_000 });
    const reviewText = await review.getText();
    assert.ok(reviewText.includes(definitionName), "the review step does not show the definition name");
    assert.ok(reviewText.includes("git status --porcelain"), "the review step does not show the verification command");
    await capture("07-wizard-review");

    await (await dialogButton("保存")).click();
    await globalThis.browser.waitUntil(
      async () => !(await (await dialog()).isExisting()),
      { timeout: 20_000, timeoutMsg: "The wizard did not close after saving." },
    );

    const definitions = await invoke(({ core }) => core.invoke("list_loop_definitions"));
    const saved = definitions.find((entry) => entry.name === definitionName);
    assert.ok(saved, "the wizard-created definition is not listed by the backend");
    assert.equal(saved.baseBranch, "main");
    assert.deepEqual(saved.acceptanceCriteria, ["seed.txt 仍然存在"]);
    assert.equal(saved.verificationCommands[0].program, "git");
    assert.deepEqual(saved.verificationCommands[0].args, ["status", "--porcelain"]);

    const overview = await globalThis.$('[role="main"]');
    await globalThis.browser.waitUntil(
      async () => (await overview.getText()).includes(definitionName),
      { timeout: 20_000, timeoutMsg: "The overview never showed the saved definition." },
    );
    // The roles section resolves registry display names, and read-only labels drop the editor's
    // per-line guidance.
    await globalThis.browser.waitUntil(async () => {
      const text = await overview.getText();
      return text.includes("Codex CLI") && text.includes("Claude Code");
    }, { timeout: 20_000, timeoutMsg: "The overview kept showing raw agent ids instead of display names." });
    assert.ok(!(await overview.getText()).includes("（每行一项）"), "the overview still shows editing-time labels");
    await capture("08-overview");
  });

  globalThis.it("opens preflight from the overview, reports every check, and starts nothing", async () => {
    const start = await globalThis.$('//*[@role="main"]//button[normalize-space(.)="检查并启动"]');
    await start.waitForClickable({ timeout: 20_000 });
    await start.click();
    const panel = await dialog();
    await panel.waitForExist({ timeout: 20_000 });
    await globalThis.browser.waitUntil(
      async () => !(await panel.getText()).includes("正在检查就绪状态"),
      { timeout: 30_000, timeoutMsg: "Preflight never finished checking." },
    );
    const text = await panel.getText();
    for (const check of ["定义已启用", "项目可用", "基础分支可用", "执行智能体符合要求", "验证智能体符合要求", "验证命令有效", "路径范围有效", "没有活动运行冲突"]) {
      assert.ok(text.includes(check), `preflight does not report the "${check}" check`);
    }
    await capture("09-preflight");

    const close = await globalThis.$('//*[@role="dialog"]//button[@aria-label="关闭就绪检查"]');
    await close.click();
    await globalThis.browser.waitUntil(
      async () => !(await (await dialog()).isExisting()),
      { timeout: 10_000, timeoutMsg: "The preflight dialog did not close." },
    );

    const runs = await invoke(({ core }) => core.invoke("list_loop_runs", { definitionId: null }));
    assert.equal(runs.length, 0, "opening preflight must not start a run");
  });

  globalThis.it("reopens the saved definition in the editor with every field intact", async () => {
    const edit = await globalThis.$('//*[@role="main"]//button[normalize-space(.)="编辑"]');
    await edit.waitForClickable({ timeout: 20_000 });
    await edit.click();
    await (await dialog()).waitForExist({ timeout: 20_000 });
    const name = await fieldControl("名称", "input");
    assert.equal(await name.getValue(), definitionName, "the editor lost the definition name");
    const goal = await fieldControl("目标", "textarea");
    assert.equal(await goal.getValue(), "保持 fixture 仓库处于绿色状态。", "the editor lost the goal");
    await capture("10-editor-reopened");
    await (await globalThis.$('//*[@role="dialog"]//button[@aria-label="关闭循环编辑器"]')).click();
    await globalThis.browser.waitUntil(
      async () => !(await (await dialog()).isExisting()),
      { timeout: 10_000, timeoutMsg: "The editor did not close." },
    );
  });

  globalThis.after(async () => {
    try {
      const definitions = await invoke(({ core }) => core.invoke("list_loop_definitions"));
      for (const entry of definitions.filter((item) => item.name.startsWith("ui-loop-"))) {
        await invoke(({ core }, definitionId) => core.invoke("delete_loop_definition", { definitionId }), entry.id);
      }
    } catch (error) {
      globalThis.console.warn(`Loop UI cleanup failed: ${error}`);
    }
  });
});
