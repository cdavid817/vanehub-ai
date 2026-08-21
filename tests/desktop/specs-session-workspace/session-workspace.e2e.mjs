import assert from "node:assert/strict";
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
      const tab = await globalThis.$(`//*[@role="tablist" and @aria-label="会话工作区"]//*[@role="tab"][normalize-space(.)="${name}"]`);
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
});
