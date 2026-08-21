import assert from "node:assert/strict";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

// The Settings shell parks non-current pages with `hidden` rather than unmounting them, so the
// visible panel is the one without the attribute. Same selector ui-settings.e2e.mjs uses.
const PANEL = "main > div > section > div:not([hidden])";

/**
 * The CLI Management page, driven through its own controls.
 *
 * `native-flows` already drives `install_cli_version` over IPC. What that cannot show is whether
 * the page a person actually uses reflects the result -- a command can install perfectly while the
 * card still reads "not installed", and no IPC-level test notices.
 *
 * Selectors are structural. The default interface language on this host is zh-CN, so anything
 * spelled as visible text is one translation away from matching nothing; the cards carry
 * `data-cli-agent` (cli-environment-card.tsx:81), which is an identifier rather than a label.
 */
async function navigate(path) {
  await globalThis.browser.execute((target) => {
    globalThis.history.pushState({}, "", target);
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  }, path);
}

async function openCliManagement() {
  await navigate("/settings?section=providers");
  await globalThis.browser.waitUntil(async () => {
    const panels = await globalThis.$$(PANEL);
    if (panels.length !== 1) return false;
    // `LazyFeature` fetches the page chunk on first visit, so the panel exists before its content
    // does; a card carrying `data-cli-agent` is the page itself having arrived.
    return await globalThis.browser.execute(
      (scope) => globalThis.document.querySelector(`${scope} [data-cli-agent]`) !== null,
      PANEL,
    );
  }, { timeout: 60_000, timeoutMsg: "The CLI Management page never became the visible panel." });
}

const cliCard = (agentId) => globalThis.$(`${PANEL} [data-cli-agent="${agentId}"]`);

/**
 * Detection is not run on a fresh data directory until something asks for it, so the catalogue
 * reports no version for a CLI that is installed on the host until this settles. Without it the
 * opencode cases below skip as "not installed" against a binary that is sitting on disk.
 */
async function refreshDetections() {
  const refresh = await invoke(({ core }) => core.invoke("refresh_cli_detections"));
  if (!refresh?.id) return;
  await globalThis.browser.waitUntil(async () => {
    const status = await invoke(
      ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
      refresh.id,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status);
  }, { timeout: 120_000, interval: 2_000, timeoutMsg: "CLI detection refresh never settled." });
}

globalThis.describe("VaneHub AI desktop CLI Management page", () => {
  globalThis.before(async () => {
    await globalThis.browser.refresh();
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("renders a card for every managed CLI Agent", async () => {
    await refreshDetections();
    await openCliManagement();
    const tools = await invoke(({ core }) => core.invoke("list_cli_tools"));
    assert.ok(tools.length > 0, "the CLI catalogue is empty");

    const missing = [];
    for (const tool of tools) {
      const card = await cliCard(tool.agentId);
      if (!(await card.isExisting())) missing.push(tool.agentId);
    }
    // Reported together: a page that lost its whole list is a different problem from one that
    // lost a single Agent, and one assertion per Agent would only ever show the first.
    assert.deepEqual(missing, [], `CLI Agents in the catalogue with no card on the page: ${missing}`);
  });

  globalThis.it("shows opencode's detected version on its card after an in-app install", async function opencodeCard() {
    await refreshDetections();
    await openCliManagement();
    const tools = await invoke(({ core }) => core.invoke("list_cli_tools"));
    const opencode = tools.find((tool) => tool.agentId === "opencode");
    assert.ok(opencode, "opencode is missing from the CLI catalogue");

    if (!opencode.currentVersion) {
      blocked.push("opencode card: nothing is installed on this host, so there is no version to render");
      this.skip();
    }

    // The page has to be showing what detection found, not a placeholder. This is the assertion
    // `native-flows` cannot make from the command side.
    const card = await cliCard("opencode");
    await card.waitForDisplayed({ timeout: 30_000 });
    const text = await card.getText();
    assert.ok(
      text.includes(opencode.currentVersion),
      `the opencode card does not show its detected version ${opencode.currentVersion}: ${text}`,
    );
  });

  globalThis.it("agrees with the Agent registry about whether opencode can be used", async function detectionAgreement() {
    await refreshDetections();
    const tools = await invoke(({ core }) => core.invoke("list_cli_tools"));
    const opencode = tools.find((tool) => tool.agentId === "opencode");
    if (!opencode?.currentVersion) {
      blocked.push("detection agreement: opencode is not installed on this host");
      this.skip();
    }

    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const agent = agents.find((entry) => entry.id === "opencode");
    assert.ok(agent, "opencode is missing from the Agent registry");

    // Two resolvers, one question. CLI detection searches a candidate list that includes the
    // directories the installers actually write to -- `~/.opencode/bin` among them
    // (tooling/cli/infrastructure/candidates.rs:104-121). Agent availability asks `which`
    // (agent_runtime/infrastructure/availability.rs:39), and so does launch
    // (infrastructure/process_adapter.rs:1367). An Agent VaneHub installed itself, into a
    // directory it knows by name, is therefore reported installed on this page and unavailable
    // everywhere else -- and cannot be launched, because launch resolves the same way availability
    // does.
    //
    // Asserted as agreement rather than as either answer: whichever way it is fixed -- teaching
    // availability and launch to use the resolved path, or installing somewhere already on PATH --
    // the two have to say the same thing, and today they do not.
    assert.equal(
      agent.availabilityState,
      "available",
      `CLI Management reports opencode installed at ${opencode.currentVersion} (active path `
      + `${opencode.activeInstallationPath ?? "unreported"}), while the Agent registry reports `
      + `${agent.availabilityState}: ${agent.unavailableReason ?? "no reason given"}. `
      + "Installed-but-unusable is not a state a person can act on from either page.",
    );
  });

  globalThis.after(async () => {
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
