import assert from "node:assert/strict";
import {
  assertNoFatalError,
  bootDesktopUi,
  clickWorkspaceTab,
  createSessionButton,
  createWorkspaceFolder,
  dialog,
  submitCreateSession,
} from "../helpers/native-ui.mjs";

const TABS = ["工作区", "变更", "文档", "文件", "终端记录", "Shell", "日志", "链路", "报告"];
let workspaceSessionId = null;

globalThis.describe("VaneHub AI desktop session workspace", () => {
  globalThis.after(async () => {
    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
  });

  globalThis.it("renders every workspace tab's own content in the desktop runtime", async function () {
    this.timeout(300000);
    const root = await bootDesktopUi();
    const folder = await createWorkspaceFolder("vanehub-workspace-");

    const opener = await createSessionButton();
    await opener.waitForClickable({ timeout: 30000 });
    await opener.click();
    await (await dialog()).waitForExist({ timeout: 20000 });
    await submitCreateSession({ projectPath: folder, title: "工作区标签原生验证", agentId: "opencode" });
    workspaceSessionId = await globalThis.browser.waitUntil(async () => {
      const sessions = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_sessions"));
      return sessions.find((session) => session.title === "工作区标签原生验证")?.id ?? false;
    }, { timeout: 30000, timeoutMsg: "The workspace session id was not persisted." });

    const tablist = await globalThis.$('[role="tablist"][aria-label="会话工作区"]');
    await tablist.waitForExist({ timeout: 30000 });
    const tabs = await globalThis.$$('[role="tablist"][aria-label="会话工作区"] [role="tab"]');
    assert.equal(tabs.length, TABS.length, `expected ${TABS.length} workspace tabs`);

    const rendered = [];
    for (const name of TABS) {
      // Matched on `title`, not on the button's text. The text also carries the evidence badge —
      // a count, a floor, or a placeholder glyph — so an equality match on it finds nothing the
      // moment a session has work to report, and a `contains` match would find the wrong tab.
      const tab = await clickWorkspaceTab(name, 20000);

      // The visible panel has to be this tab's panel, not merely some panel: a workspace that
      // switches the selected tab without switching the panel looks correct in a screenshot.
      const panel = await globalThis.browser.waitUntil(async () => {
        const snapshot = await globalThis.browser.execute(() => {
          const visible = globalThis.document.querySelector('[id^="session-tab-panel-"]:not(.hidden)');
          if (!visible) return null;
          return {
            id: visible.id,
            labelledBy: visible.getAttribute("aria-labelledby"),
            text: visible.innerText.trim().length,
            sample: visible.innerText.trim().slice(0, 120),
            nodes: visible.querySelectorAll("*").length,
          };
        });
        // Every panel behind the first is a lazy chunk. Accepting its loading placeholder would
        // pass on a client where no chunk ever resolves, which is the failure worth catching.
        return snapshot && snapshot.text > 0 && !snapshot.sample.includes("正在加载") ? snapshot : false;
      }, { timeout: 30000, timeoutMsg: `The ${name} panel never rendered past its loading placeholder.` });

      const controls = await tab.getAttribute("aria-controls");
      assert.equal(panel.id, controls, `the visible panel does not belong to the ${name} tab`);
      rendered.push({ name, text: panel.text, nodes: panel.nodes, sample: panel.sample });
    }

    globalThis.console.warn("WORKSPACE_TABS " + JSON.stringify(rendered));
    await assertNoFatalError(root);
  });

  globalThis.it("reads terminal history through the native record query", async function () {
    this.timeout(300000);
    const root = await bootDesktopUi();

    await clickWorkspaceTab("终端记录", 20000);
    const views = await globalThis.$('[role="tablist"][aria-label="执行记录视图"]');
    await views.waitForExist({ timeout: 20000 });

    // Straight through the registered Tauri command, with no client in the way. A panel that
    // rendered rows the command cannot produce would be reading a fixture, and on the desktop
    // runtime that is exactly the confusion this console exists to remove.
    const nativeCount = async () =>
      await globalThis.browser.tauri.execute(async ({ core }) => {
        // Session-wide rather than pinned to one id: the assertion is about the journal not moving,
        // and reading every session makes an accidental append anywhere visible.
        const page = await core.invoke("list_execution_records", {
          scope: {},
          filters: null,
          cursor: null,
          limit: 100,
        });
        return page.items.length;
      });

    const before = await nativeCount();
    assert.equal(typeof before, "number", "the native record query must answer");

    // Legacy activity is projected from loaded messages and writes nothing. If it appended to the
    // journal, opening its view would change what the native query returns.
    const legacy = await globalThis.$('[data-testid="execution-record-view-legacy"]');
    await legacy.waitForClickable({ timeout: 20000 });
    await legacy.click();
    await (await globalThis.$('[data-testid="legacy-source-notice"]')).waitForExist({
      timeout: 20000,
    });
    assert.equal(
      await nativeCount(),
      before,
      "viewing legacy activity changed the native evidence journal",
    );

    // Hidden means paused, not unmounted: the view the reader chose is still chosen when they
    // come back, which is the Group 5 retention rule applied to this panel.
    await clickWorkspaceTab("报告", 20000);
    await clickWorkspaceTab("终端记录", 20000);
    const stillLegacy = await globalThis.$('[data-testid="legacy-source-notice"]');
    assert.ok(
      await stillLegacy.isExisting(),
      "the terminal history view was reset by a round trip through another tab",
    );

    await assertNoFatalError(root);
  });

  globalThis.it("renders the trace waterfall in both visual styles", async function () {
    this.timeout(300000);
    const root = await bootDesktopUi();
    assert.ok(workspaceSessionId, "the workspace session from the first test is unavailable");

    const traceSpanCount = async () => {
      const page = await globalThis.browser.tauri.execute(({ core }, sessionId) => (
        core.invoke("list_execution_runs", {
          request: { limit: 20, pageToken: null },
          sessionId,
        })
      ), workspaceSessionId);
      if (!page.items.length) return 0;
      const timeline = await globalThis.browser.tauri.execute(({ core }, runId) => (
        core.invoke("get_execution_timeline", { runId })
      ), page.items[0].runId);
      return timeline.spans.length;
    };
    if (await traceSpanCount() === 0) {
      const request = {
        sessionId: workspaceSessionId,
        content: "VANEHUB_E2E_TRACE",
        config: {
          agentId: "opencode",
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
        runner: null,
      };
      for (let attempt = 0; attempt < 3; attempt += 1) {
        try {
          await globalThis.browser.tauri.execute(
            ({ core }, payload) => core.invoke("send_message", payload),
            request,
          );
          break;
        } catch (error) {
          if (!String(error).includes("database is locked") || attempt === 2) throw error;
          await globalThis.browser.pause(250);
        }
      }
    }
    await globalThis.browser.waitUntil(async () => (await traceSpanCount()) > 0, {
      timeout: 60000,
      timeoutMsg: "The deterministic CLI turn produced no trace spans.",
    });

    await clickWorkspaceTab("链路", 20000);

    // The waterfall is the panel most likely to become a picture with no text in it, so the
    // assertion is on its landmarks rather than on it having rendered *something*: a named scroll
    // container, and the legend and filter groups that name what the colours mean.
    const landmarks = async () => await globalThis.browser.execute(() => {
      const panel = globalThis.document.querySelector('[id^="session-tab-panel-"]:not(.hidden)');
      if (!panel) return null;
      return {
        waterfall: panel.querySelectorAll('[role="application"]').length,
        groups: panel.querySelectorAll('[role="group"]').length,
        text: panel.innerText.trim().length,
      };
    });

    for (const theme of ["futuristic", "minimal"]) {
      // Driven through the document attribute the settings provider sets, which is what actually
      // selects the stylesheet. A theme decides how a row looks, never whether it exists.
      await globalThis.browser.execute((next) => {
        globalThis.document.documentElement.dataset.theme = next;
      }, theme);
      const seen = await globalThis.browser.waitUntil(async () => {
        const snapshot = await landmarks();
        return snapshot && snapshot.text > 0 ? snapshot : false;
      }, {
        timeout: 30000,
        timeoutMsg: `The trace panel rendered nothing under the ${theme} style.`,
      });
      assert.ok(seen.waterfall >= 1, `no named waterfall container under the ${theme} style`);
      assert.ok(seen.groups >= 2, `the legend and filter groups are missing under the ${theme} style`);
      globalThis.console.warn("TRACE_PANEL " + JSON.stringify({ theme, ...seen }));
    }

    await globalThis.browser.execute(() => {
      globalThis.document.documentElement.dataset.theme = "futuristic";
    });
    await assertNoFatalError(root);
  });
});
