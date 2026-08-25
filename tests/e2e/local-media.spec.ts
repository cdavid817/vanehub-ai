import { expect, test, type Page } from "@playwright/test";

/**
 * Web-mode behaviour for the three local media actions.
 *
 * The point of these tests is that the browser build tells the truth. Local OCR, a microphone, and
 * a speech engine are native capabilities; the Web adapter is deliberately inert, and what has to
 * be verified here is that it stays visibly, explicably unavailable rather than pretending.
 */
async function openLocalMediaSettings(page: Page) {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /^(本地媒体|Local Media)$/ }).click();
  await expect(
    page.getByRole("heading", { name: /^(本地媒体|Local Media)$/, level: 2 }),
  ).toBeVisible();
}

test.describe("Local media settings in Web mode", () => {
  test("presents all three engines and says plainly that they need the desktop client", async ({
    page,
  }) => {
    await openLocalMediaSettings(page);

    await expect(page.getByTestId("local-media-card-ocr")).toBeVisible();
    await expect(page.getByTestId("local-media-card-stt")).toBeVisible();
    await expect(page.getByTestId("local-media-card-tts")).toBeVisible();
    await expect(page.getByText(/这些能力需要桌面客户端/)).toBeVisible();

    // Checking would start a worker, which the browser cannot do; the control says so rather than
    // failing after the click.
    await expect(page.getByTestId("local-media-probe-ocr")).toBeDisabled();
    await expect(page.getByTestId("local-media-probe-stt")).toBeDisabled();
    await expect(page.getByTestId("local-media-probe-tts")).toBeDisabled();
  });

  test("offers no way to install or download an engine or a model", async ({ page }) => {
    await openLocalMediaSettings(page);

    // The product configures software the user installed themselves. A download affordance here
    // would be a promise the feature never keeps.
    await expect(page.getByRole("button", { name: /下载|安装|Download|Install/ })).toHaveCount(0);
  });

  test("states the local-only guarantee as a claim about this feature, not the operating system", async ({
    page,
  }) => {
    await openLocalMediaSettings(page);

    const note = page.getByText(/不会离开本机/);
    await expect(note).toBeVisible();
    await expect(note).toContainText("并不代表操作系统层面的沙箱");
  });
});

/**
 * The media actions live in the structured composer, which only renders for an API-mode or
 * multi-seat session. A multi-seat session is the one the Web/mock adapter can create end to end.
 */
async function openStructuredComposer(page: Page) {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: /新建/ }).click();
  const projectPath = page.getByPlaceholder(/code.*project/);
  await expect(async () => {
    await projectPath.fill("D:\\example-workspace");
    await projectPath.press("Tab");
    await expect(page.getByRole("button", { name: "创建", exact: true })).toBeEnabled({
      timeout: 1_000,
    });
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
  await page.getByPlaceholder("新会话").fill("本地媒体会话");
  await page.getByRole("button", { name: "创建", exact: true }).click();
  await expect(page.getByTestId("composer-media-actions")).toBeVisible();
}

test.describe("Composer media actions in Web mode", () => {
  test("keeps the three actions visible and disabled instead of hiding them", async ({ page }) => {
    await openStructuredComposer(page);

    // Hiding them would leave a user who reads about the feature with nowhere to find out why it
    // is missing. Disabled with a reason in the tooltip is the honest state.
    for (const id of [
      "composer-media-ocr",
      "composer-media-microphone",
      "composer-media-speak",
    ]) {
      const control = page.getByTestId(id);
      await expect(control).toBeVisible();
      await expect(control).toBeDisabled();
      await expect(control).toHaveAttribute("title", /桌面客户端|尚未就绪/);
    }
  });

  test("labels every icon-only action for assistive technology", async ({ page }) => {
    await openStructuredComposer(page);

    await expect(page.getByTestId("composer-media-ocr")).toHaveAttribute(
      "aria-label",
      "图片文字识别",
    );
    await expect(page.getByTestId("composer-media-microphone")).toHaveAttribute(
      "aria-label",
      "按住说话",
    );
    await expect(page.getByTestId("composer-media-speak")).toHaveAttribute(
      "aria-label",
      "朗读文本",
    );
  });

  test("never opens a microphone or changes the draft when a disabled action is pressed", async ({
    page,
  }) => {
    await openStructuredComposer(page);

    const composer = page.getByPlaceholder(/输入指令/);
    await composer.fill("draft text");
    await page.getByTestId("composer-media-microphone").click({ force: true });

    await expect(composer).toHaveValue("draft text");
    await expect(page.getByTestId("composer-media-recording")).toHaveCount(0);
  });

  test("leaves the existing send and stop controls untouched", async ({ page }) => {
    await openStructuredComposer(page);

    // The media group is an addition to the toolbar, not a replacement for it.
    const toolbar = page.getByTestId("composer-toolbar");
    await expect(toolbar.getByRole("button", { name: "发送" })).toBeVisible();
    await expect(page.getByPlaceholder(/输入指令/)).toBeEnabled();
  });
});
