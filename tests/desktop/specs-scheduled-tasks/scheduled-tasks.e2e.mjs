import assert from "node:assert/strict";
import path from "node:path";
import process from "node:process";
import {
  activeElementInside,
  assertNoFatalError,
  automationsTab,
  bootDesktopUi,
  openScheduledTasksSurface,
  selectSurfaceOption,
  surfaceButton,
  surfaceField,
} from "../helpers/native-ui.mjs";

const taskName = "WebdriverIO 定时任务验证";
const surfaceId = "scheduled-tasks-panel";

globalThis.describe("VaneHub AI scheduled tasks", () => {
  globalThis.after(async () => {
    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
  });

  globalThis.it("persists the scheduled task lifecycle through the native desktop UI", async function () {
    this.timeout(240000);
    const root = await bootDesktopUi();
    await openScheduledTasksSurface();

    // Page content, not a modal: nothing with dialog semantics may be open, the route names the
    // tab, and focus lands on the surface's first control (the task name) rather than "somewhere
    // inside a dialog".
    assert.equal(await (await globalThis.$('[role="dialog"]')).isExisting(), false, "scheduled tasks opened a dialog");
    assert.match(await globalThis.browser.getUrl(), /\/workspace\/automations\/scheduled/);
    const name = await surfaceField(surfaceId, "任务名称");
    await globalThis.browser.waitUntil(async () => name.isFocused(), {
      timeout: 10000,
      timeoutMsg: "focus did not move to the first control of the scheduled-task surface",
    });
    assert.ok(await activeElementInside(surfaceId), "focus is not inside the scheduled-task surface");

    // An unsent draft survives a trip to another tab: the surface is hidden, not unmounted.
    await name.setValue(taskName);
    await (await automationsTab("循环工程")).click();
    await globalThis.browser.waitUntil(async () => (await globalThis.browser.getUrl()).includes("/workspace/automations/loops"), {
      timeout: 10000,
      timeoutMsg: "The Loops tab did not become the route.",
    });
    await (await automationsTab("定时任务")).click();
    await globalThis.browser.waitUntil(async () => (await (await surfaceField(surfaceId, "任务名称")).getValue()) === taskName, {
      timeout: 10000,
      timeoutMsg: "The scheduled-task draft did not survive a tab switch.",
    });

    const content = await surfaceField(surfaceId, "任务内容");
    await content.setValue("检查项目状态并输出摘要");
    await selectSurfaceOption(surfaceId, "Agent 工具", "opencode");
    await selectSurfaceOption(surfaceId, "执行频率", "minutes");
    const interval = await surfaceField(surfaceId, "间隔");
    await interval.setValue("30");

    const create = await surfaceButton(surfaceId, "创建任务");
    await globalThis.browser.waitUntil(async () => create.isEnabled(), {
      timeout: 20000,
      timeoutMsg: "A valid scheduled task never became submittable.",
    });
    await create.click();

    const created = await waitForNativeTask();
    assert.equal(created.agentId, "opencode");
    assert.deepEqual(created.frequency, { kind: "minutes", interval: 30 });
    const row = await globalThis.$(`[data-scheduled-task-id="${created.id}"]`);
    await row.waitForExist({ timeout: 20000 });
    assert.match(await row.getText(), /每 30 分钟/);

    const disable = await globalThis.$(`//*[@role="switch" and @aria-label="停用任务“${taskName}”"]`);
    await disable.click();
    await waitForNativeEnabled(false);
    const enable = await globalThis.$(`//*[@role="switch" and @aria-label="启用任务“${taskName}”"]`);
    await enable.waitForExist({ timeout: 20000 });
    await enable.click();
    await waitForNativeEnabled(true);

    const resultDir = process.env.VANEHUB_DESKTOP_RESULT_DIR;
    if (resultDir) await globalThis.browser.saveScreenshot(path.join(resultDir, "screenshots", "scheduled-tasks-inline.png"));

    const remove = await globalThis.$(`//*[@data-testid="${surfaceId}"]//button[@aria-label="删除定时任务“${taskName}”？"]`);
    await remove.click();
    await (await surfaceButton(surfaceId, "确认删除")).click();
    await globalThis.browser.waitUntil(async () => !(await nativeTasks()).some((task) => task.id === created.id), {
      timeout: 20000,
      timeoutMsg: "The scheduled task remained in native persistence after deletion.",
    });
    await globalThis.browser.waitUntil(async () => {
      const deletedRow = await globalThis.$(`[data-scheduled-task-id="${created.id}"]`);
      return !(await deletedRow.isExisting());
    }, { timeout: 10000, timeoutMsg: "The deleted scheduled-task row remained visible." });
    await assertNoFatalError(root);
  });
});

const nativeTasks = () => globalThis.browser.tauri.execute(({ core }) => core.invoke("list_scheduled_tasks"));

async function waitForNativeTask() {
  let created;
  await globalThis.browser.waitUntil(async () => {
    created = (await nativeTasks()).find((task) => task.name === taskName);
    return Boolean(created);
  }, { timeout: 30000, timeoutMsg: "The scheduled task was never persisted natively." });
  return created;
}

async function waitForNativeEnabled(enabled) {
  await globalThis.browser.waitUntil(async () => {
    const task = (await nativeTasks()).find((candidate) => candidate.name === taskName);
    return task?.enabled === enabled;
  }, { timeout: 20000, timeoutMsg: `Native enabled state did not become ${enabled}.` });
}
