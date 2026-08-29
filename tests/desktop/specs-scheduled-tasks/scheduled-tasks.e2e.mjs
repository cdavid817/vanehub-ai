import assert from "node:assert/strict";
import path from "node:path";
import process from "node:process";
import {
  activeElementInsideDialog,
  assertNoFatalError,
  bootDesktopUi,
  dialog,
  dialogButton,
  dialogField,
  scheduledTasksButton,
  selectDialogOption,
} from "../helpers/native-ui.mjs";

const taskName = "WebdriverIO 定时任务验证";

globalThis.describe("VaneHub AI scheduled tasks", () => {
  globalThis.after(async () => {
    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
  });

  globalThis.it("persists the scheduled task lifecycle through the native desktop UI", async function () {
    this.timeout(240000);
    const root = await bootDesktopUi();
    const opener = await scheduledTasksButton();
    await opener.waitForClickable({ timeout: 30000 });
    await opener.click();

    const opened = await dialog();
    await opened.waitForExist({ timeout: 20000 });
    assert.equal(await opened.getAttribute("aria-modal"), "true");
    assert.ok(await activeElementInsideDialog(), "focus never entered the scheduled-task dialog");
    await globalThis.browser.keys(["Escape"]);
    await opened.waitForExist({ timeout: 10000, reverse: true });
    assert.ok(await opener.isFocused(), "focus did not return to the Scheduled Tasks activity button");

    await opener.click();
    const name = await dialogField("任务名称");
    const content = await dialogField("任务内容");
    await name.setValue(taskName);
    await content.setValue("检查项目状态并输出摘要");
    await selectDialogOption("Agent 工具", "opencode");
    await selectDialogOption("执行频率", "minutes");
    const interval = await dialogField("间隔");
    await interval.setValue("30");

    const create = await dialogButton("创建任务");
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
    if (resultDir) await globalThis.browser.saveScreenshot(path.join(resultDir, "screenshots", "scheduled-tasks-optimized.png"));

    const remove = await globalThis.$(`//*[@role="dialog"]//button[@aria-label="删除定时任务“${taskName}”？"]`);
    await remove.click();
    await (await dialogButton("确认删除")).click();
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
