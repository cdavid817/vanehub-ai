import { expect, test, type Page } from "@playwright/test";

/**
 * Local media driven by the deterministic fake.
 *
 * These are the state transitions the honest Web adapter can never show, because it refuses every
 * capability rather than simulating one. The fake exists only in this build; `local-media-fake-
 * boundary.test.ts` proves a shipped bundle contains none of it.
 *
 * Nothing here touches a real engine, a real microphone, or a real speaker, and nothing here is
 * evidence that any of those work.
 */
type Outcome = string;

async function control(page: Page, method: string, ...args: unknown[]) {
  await page.evaluate(
    ([name, callArgs]) => {
      const handle = (window as unknown as Record<string, Record<string, (...rest: unknown[]) => void>>)
        .__vanehubLocalMediaFake;
      if (!handle) throw new Error("the local-media fake is not installed in this build");
      handle[name as string](...(callArgs as unknown[]));
    },
    [method, args] as const,
  );
}

async function calls(page: Page): Promise<Record<string, number>> {
  return page.evaluate(() => {
    const handle = (window as unknown as Record<string, { calls(): Record<string, number> }>)
      .__vanehubLocalMediaFake;
    return handle.calls();
  });
}

/** A multi-seat session is the one structured composer the Web build can reach end to end. */
async function openComposer(page: Page) {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: /新建/ }).click();
  const projectPath = page.getByPlaceholder(/code.*project/);
  await expect(async () => {
    await projectPath.fill("D:\\example-workspace");
    await projectPath.press("Tab");
    await expect(page.getByRole("button", { name: "创建", exact: true })).toBeEnabled({ timeout: 1_000 });
  }).toPass({ timeout: 10_000 });
  await page.getByRole("button", { name: /多个 Agent 在同一会话里协作/ }).click();
  const seatRows = page.locator("div.ucd-list-row").filter({
    has: page.getByRole("button", { name: /删除席位/ }),
  });
  await expect(seatRows).toHaveCount(2);
  for (let index = 0; index < 2; index += 1) {
    const select = seatRows.nth(index).getByRole("combobox", { name: "Agent" });
    const value = await select.locator("option").nth(index).getAttribute("value");
    if (value) await select.selectOption(value);
  }
  await page.getByPlaceholder("新会话").fill("本地媒体 fake 会话");
  await page.getByRole("button", { name: "创建", exact: true }).click();
  await expect(page.getByTestId("composer-media-actions")).toBeVisible();
}

function composer(page: Page) {
  return page.getByPlaceholder(/输入指令/);
}

test.describe("local media driven by the deterministic fake", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.clear();
    });
  });

  test("enables each action from its own engine's readiness", async ({ page }) => {
    await openComposer(page);
    await control(page, "setEngineUnavailable", "ocr", "MODEL_NOT_FOUND");
    await page.reload({ waitUntil: "domcontentloaded" });
    await openComposer(page);

    // One unconfigured engine must never take the others down with it.
    await expect(page.getByTestId("composer-media-ocr")).toBeEnabled();
    await expect(page.getByTestId("composer-media-microphone")).toBeEnabled();
  });

  test("recognizes text, reviews it, and appends only on confirmation", async ({ page }) => {
    await openComposer(page);
    await composer(page).fill("existing draft");

    await page.getByTestId("composer-media-ocr").click();
    await expect(page.getByTestId("composer-ocr-review")).toBeVisible();
    // The draft is untouched while the review is open: this is where the text becomes the user's.
    await expect(composer(page)).toHaveValue("existing draft");

    await page.getByTestId("composer-ocr-append").click();
    await expect(composer(page)).toHaveValue(/existing draft\n\nfixture recognized line one/);
  });

  test("keeps an edit made in the review", async ({ page }) => {
    await openComposer(page);
    await page.getByTestId("composer-media-ocr").click();
    const text = page.getByTestId("composer-ocr-text");
    await text.fill("edited in review");
    await page.getByTestId("composer-ocr-append").click();

    await expect(composer(page)).toHaveValue("edited in review");
  });

  test("discards the staged source when the review is cancelled", async ({ page }) => {
    await openComposer(page);
    await page.getByTestId("composer-media-ocr").click();
    await page.getByRole("button", { name: "放弃" }).click();

    await expect(page.getByTestId("composer-ocr-review")).toHaveCount(0);
    await expect(composer(page)).toHaveValue("");
    expect((await calls(page)).discardStagedOcrSource).toBe(1);
  });

  test("starts nothing when the picker is cancelled", async ({ page }) => {
    await openComposer(page);
    await control(page, "scriptOcr", "picker-cancelled" satisfies Outcome);
    await page.getByTestId("composer-media-ocr").click();

    await expect(page.getByTestId("composer-ocr-review")).toHaveCount(0);
    expect((await calls(page)).startOcr).toBeUndefined();
  });

  test("reports no recognized text as information rather than failure", async ({ page }) => {
    await openComposer(page);
    await control(page, "scriptOcr", "no-text");
    await page.getByTestId("composer-media-ocr").click();

    await expect(page.getByTestId("composer-ocr-review")).toBeVisible();
    await expect(page.getByText("没有识别到文字")).toBeVisible();
    await expect(page.getByTestId("composer-ocr-append")).toBeDisabled();
  });

  test("surfaces a recognition failure as localized copy", async ({ page }) => {
    await openComposer(page);
    await control(page, "scriptOcr", "failure");
    await page.getByTestId("composer-media-ocr").click();

    await expect(page.getByTestId("composer-media-failure")).toContainText("引擎进程意外退出");
    await expect(page.getByTestId("composer-ocr-review")).toHaveCount(0);
  });

  test("rejects a staging failure before any operation starts", async ({ page }) => {
    await openComposer(page);
    await control(page, "scriptOcr", "staging-failure");
    await page.getByTestId("composer-media-ocr").click();

    await expect(page.getByTestId("composer-media-failure")).toContainText("PNG");
    expect((await calls(page)).startOcr).toBeUndefined();
  });

  test("shows progress without blocking the composer", async ({ page }) => {
    await openComposer(page);
    await control(page, "setOperationDelay", 4);
    await page.getByTestId("composer-media-ocr").click();

    // The textarea stays usable while the engine works; the result joins whatever is typed.
    await composer(page).fill("typed while working");
    await expect(page.getByTestId("composer-ocr-review")).toBeVisible({ timeout: 15_000 });
    await page.getByTestId("composer-ocr-append").click();
    await expect(composer(page)).toHaveValue(/typed while working/);
  });

  test("appends a transcript to the latest draft and never sends", async ({ page }) => {
    await openComposer(page);
    const microphone = page.getByTestId("composer-media-microphone");
    await microphone.dispatchEvent("pointerdown", { button: 0, pointerId: 1 });
    await expect(page.getByTestId("composer-media-recording")).toBeVisible();
    await composer(page).fill("typed during capture");
    await microphone.dispatchEvent("pointerup", { button: 0, pointerId: 1 });

    await expect(composer(page)).toHaveValue("typed during capture fixture transcript");
    // Nothing may auto-send: the transcript is a draft edit, not a submission.
    await expect(page.getByTestId("composer-toolbar").getByRole("button", { name: "发送" })).toBeVisible();
  });

  test("cancels a hold on Escape without transcribing", async ({ page }) => {
    await openComposer(page);
    const microphone = page.getByTestId("composer-media-microphone");
    await microphone.dispatchEvent("pointerdown", { button: 0, pointerId: 1 });
    await expect(page.getByTestId("composer-media-recording")).toBeVisible();
    await page.keyboard.press("Escape");

    await expect(page.getByTestId("composer-media-recording")).toHaveCount(0);
    await expect(composer(page)).toHaveValue("");
    const observed = await calls(page);
    expect(observed.cancelRecording).toBe(1);
    expect(observed.stopRecordingAndTranscribe).toBeUndefined();
  });

  test("treats an empty utterance as information and leaves the draft alone", async ({ page }) => {
    await openComposer(page);
    await control(page, "scriptStt", "no-speech");
    await composer(page).fill("kept");
    const microphone = page.getByTestId("composer-media-microphone");
    await microphone.dispatchEvent("pointerdown", { button: 0, pointerId: 1 });
    await microphone.dispatchEvent("pointerup", { button: 0, pointerId: 1 });

    await expect(composer(page)).toHaveValue("kept");
    await expect(page.getByTestId("composer-media-failure")).toHaveCount(0);
  });

  test("surfaces a denied microphone before any recording indicator appears", async ({ page }) => {
    await openComposer(page);
    await control(page, "scriptStt", "start-failure");
    await page
      .getByTestId("composer-media-microphone")
      .dispatchEvent("pointerdown", { button: 0, pointerId: 1 });

    await expect(page.getByTestId("composer-media-failure")).toContainText("麦克风");
    await expect(page.getByTestId("composer-media-recording")).toHaveCount(0);
  });

  test("speaks the selection in preference to the whole draft", async ({ page }) => {
    await openComposer(page);
    await composer(page).fill("alpha beta gamma");
    // A real caret movement rather than a synthetic `select` event: React listens through its own
    // delegation, and a dispatched event never reaches `onSelect`.
    await composer(page).click();
    await page.keyboard.press("Home");
    for (let index = 0; index < 6; index += 1) await page.keyboard.press("ArrowRight");
    for (let index = 0; index < 4; index += 1) await page.keyboard.press("Shift+ArrowRight");
    await page.getByTestId("composer-media-speak").click();

    // Asserting what was synthesized, not a transient control state: the rule under test is that
    // a selection wins over the whole draft.
    await expect
      .poll(async () =>
        page.evaluate(() => {
          const handle = (window as unknown as Record<string, { lastTtsText(): string | null }>)
            .__vanehubLocalMediaFake;
          return handle.lastTtsText();
        }),
      )
      .toBe("beta");
  });

  test("speaks the whole draft when nothing is selected", async ({ page }) => {
    await openComposer(page);
    await composer(page).fill("alpha beta gamma");
    await page.getByTestId("composer-media-speak").click();

    await expect
      .poll(async () =>
        page.evaluate(() => {
          const handle = (window as unknown as Record<string, { lastTtsText(): string | null }>)
            .__vanehubLocalMediaFake;
          return handle.lastTtsText();
        }),
      )
      .toBe("alpha beta gamma");
  });

  test("stops playback when pressed a second time", async ({ page }) => {
    await openComposer(page);
    await control(page, "setOperationDelay", 8);
    await composer(page).fill("speak this");
    const speak = page.getByTestId("composer-media-speak");
    await speak.click();
    await expect(speak).toHaveAttribute("aria-pressed", "true");
    await speak.click();

    expect((await calls(page)).stopPlayback).toBe(1);
  });

  test("reports a synthesis failure and returns to idle", async ({ page }) => {
    await openComposer(page);
    await control(page, "scriptTts", "failure");
    await composer(page).fill("speak this");
    await page.getByTestId("composer-media-speak").click();

    await expect(page.getByTestId("composer-media-failure")).toContainText("音频输出设备");
    await expect(page.getByTestId("composer-media-speak")).toHaveAttribute("aria-pressed", "false");
  });

  test("drops a result whose composer scope the user has left", async ({ page }) => {
    await openComposer(page);
    await control(page, "setOperationDelay", 3);
    await control(page, "setStaleScope", true);
    await page.getByTestId("composer-media-ocr").click();

    // The work completes for nobody: no review appears and the draft is untouched.
    await page.waitForTimeout(3_000);
    await expect(page.getByTestId("composer-ocr-review")).toHaveCount(0);
    await expect(composer(page)).toHaveValue("");
  });

  test("reports an expired result instead of polling a dead operation", async ({ page }) => {
    await openComposer(page);
    await control(page, "setOperationDelay", 5);
    await page.getByTestId("composer-media-ocr").click();
    await control(page, "expireResults");

    await expect(page.getByTestId("composer-media-failure")).toContainText("结果已过期");
  });

  test("shows a probe result on the settings page", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /^(本地媒体|Local Media)$/ }).click();

    // The fake reports a configured profile, so the page renders its ready state rather than the
    // native-only notice the honest Web adapter produces.
    await expect(page.getByTestId("local-media-card-ocr")).toContainText("可用");
    await page.getByTestId("local-media-probe-ocr").click();
    await expect(page.getByTestId("local-media-card-ocr")).toContainText("fixture-1.0.0");
  });
});
