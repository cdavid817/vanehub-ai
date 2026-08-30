import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import {
  assertNoFatalError,
  bootDesktopUi,
  createSessionButton,
  createWorkspaceFolder,
  dialog,
  submitCreateSession,
} from "../helpers/native-ui.mjs";

const TABS = ["工作区", "变更", "文档", "文件", "终端记录", "Shell", "日志", "链路", "报告"];

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

    const tablist = await globalThis.$('[role="tablist"][aria-label="会话工作区"]');
    await tablist.waitForExist({ timeout: 30000 });
    const tabs = await globalThis.$$('[role="tablist"][aria-label="会话工作区"] [role="tab"]');
    assert.equal(tabs.length, TABS.length, `expected ${TABS.length} workspace tabs`);

    const rendered = [];
    for (const name of TABS) {
      // Matched on `title`, not on the button's text. The text also carries the evidence badge —
      // a count, a floor, or a placeholder glyph — so an equality match on it finds nothing the
      // moment a session has work to report, and a `contains` match would find the wrong tab.
      const tab = await globalThis.$(`//*[@role="tablist" and @aria-label="会话工作区"]//*[@role="tab" and @title="${name}"]`);
      await tab.waitForClickable({ timeout: 20000 });
      await tab.click();
      await globalThis.browser.waitUntil(async () => await tab.getAttribute("aria-selected") === "true", {
        timeout: 20000,
        timeoutMsg: `The ${name} tab never became the selected tab.`,
      });

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

    const terminal = await globalThis.$(
      '//*[@role="tablist" and @aria-label="会话工作区"]//*[@role="tab" and @title="终端记录"]',
    );
    await terminal.waitForClickable({ timeout: 20000 });
    await terminal.click();
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
    const report = await globalThis.$(
      '//*[@role="tablist" and @aria-label="会话工作区"]//*[@role="tab" and @title="报告"]',
    );
    await report.waitForClickable({ timeout: 20000 });
    await report.click();
    await terminal.click();
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

    const traces = await globalThis.$(
      '//*[@role="tablist" and @aria-label="会话工作区"]//*[@role="tab" and @title="链路"]',
    );
    await traces.waitForClickable({ timeout: 20000 });
    await traces.click();

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

  globalThis.it("says part of the workspace was never searched when a real tree is too deep", async function () {
    this.timeout(300000);
    const root = await bootDesktopUi();
    // Twelve levels, against a walk that descends ten. Deep rather than wide because depth is the
    // one native ceiling a fixture can reach without writing hundreds of thousands of entries, and
    // the notice under test is the same one every other budget produces.
    const folder = await createWorkspaceFolder("vanehub-deep-workspace-");
    let deep = folder;
    for (let level = 0; level < 12; level += 1) {
      deep = join(deep, `level-${level}`);
      await mkdir(deep, { recursive: true });
      await writeFile(join(deep, "buried.txt"), "needle-at-the-bottom\n", "utf8");
    }

    const opener = await createSessionButton();
    await opener.waitForClickable({ timeout: 30000 });
    await opener.click();
    await (await dialog()).waitForExist({ timeout: 20000 });
    await submitCreateSession({ projectPath: folder, title: "深层工作区搜索验证", agentId: "opencode" });

    const files = await globalThis.$('//*[@role="tablist" and @aria-label="会话工作区"]//*[@role="tab" and @title="文件"]');
    await files.waitForClickable({ timeout: 30000 });
    await files.click();
    const openSearch = await globalThis.$('//button[@title="在文件中搜索" or @aria-label="在文件中搜索"]');
    await openSearch.waitForClickable({ timeout: 30000 });
    await openSearch.click();

    const input = await globalThis.$('[role="combobox"][aria-label="在此工作区中查找文本"]');
    await input.waitForExist({ timeout: 20000 });
    await input.setValue("needle-at-the-bottom");

    // The distinction the coverage contract exists for, against a real filesystem and the native
    // walk rather than a mock: the search found matches and still did not see everything, so it
    // must not read as an authoritative answer about the parts it never reached.
    const notice = await globalThis.browser.waitUntil(async () => {
      const text = await globalThis.browser.execute(() => {
        const panel = globalThis.document.querySelector('[role="dialog"][aria-label="在文件中搜索"]');
        return panel ? panel.innerText : "";
      });
      return text.includes("此工作区有一部分未被搜索。") ? text : false;
    }, { timeout: 60000, timeoutMsg: "the deep workspace never reported incomplete coverage" });

    globalThis.console.warn("WORKSPACE_SEARCH_COVERAGE " + JSON.stringify(notice.slice(0, 200)));
    // Escape closes the panel and cancels whatever is still running. Every spec in this file shares
    // one client, so a modal left open is the next test's missing element.
    await globalThis.browser.keys("Escape");
    await globalThis.browser.waitUntil(async () => !(await (await globalThis.$('[role="dialog"][aria-label="在文件中搜索"]')).isExisting()), {
      timeout: 20000,
      timeoutMsg: "the search panel stayed open",
    });
    await assertNoFatalError(root);
  });
});
