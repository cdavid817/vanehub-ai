import assert from "node:assert/strict";
import path from "node:path";
import process from "node:process";

const RESULT_DIR = process.env.VANEHUB_DESKTOP_RESULT_DIR;
const SCREENSHOT_DIR = path.join(RESULT_DIR, "screenshots");
const AGENT_PAGE = '[data-testid="agent-configurations-page"]';
const CODE_INTELLIGENCE_PAGE = '[data-testid="code-intelligence-page"]';

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

async function navigate(target) {
  await globalThis.browser.execute((url) => {
    globalThis.history.pushState({}, "", url);
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  }, target);
}

async function waitForBootstrap() {
  const root = await globalThis.$("#root");
  await root.waitForExist({ timeout: 120_000 });
  await globalThis.browser.waitUntil(
    async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
    { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
  );
}

async function openSettings(section, pageSelector) {
  await navigate(`/settings?section=${section}`);
  const page = await globalThis.$(pageSelector);
  await page.waitForDisplayed({ timeout: 60_000 });
  return page;
}

async function setTheme(theme) {
  await invoke(({ core }, value) => core.invoke("save_setting", {
    input: { key: "theme", value },
  }), theme);
  await globalThis.browser.waitUntil(async () => (
    await globalThis.$("html").getAttribute("data-theme")
  ) === theme, { timeout: 20_000, timeoutMsg: `Desktop theme did not change to ${theme}.` });
}

async function assertNoHorizontalOverflow(context) {
  const diagnostics = await globalThis.browser.execute(() => {
    const viewportWidth = globalThis.document.documentElement.clientWidth;
    return {
      clientWidth: viewportWidth,
      scrollWidth: globalThis.document.documentElement.scrollWidth,
      offenders: [...globalThis.document.querySelectorAll("body *")]
        .map((element) => {
          const rect = element.getBoundingClientRect();
          return {
            element: element.tagName.toLowerCase(),
            className: typeof element.className === "string" ? element.className : "",
            testId: element.getAttribute("data-testid"),
            left: Math.round(rect.left),
            right: Math.round(rect.right),
            width: Math.round(rect.width),
          };
        })
        .filter((item) => item.right > viewportWidth + 1 || item.left < -1)
        .slice(0, 12),
    };
  });
  assert.equal(
    diagnostics.scrollWidth > diagnostics.clientWidth,
    false,
    `${context} overflowed the desktop WebView horizontally: ${JSON.stringify(diagnostics)}`,
  );
}

async function selectAgent(agentId) {
  const target = await globalThis.$(`[data-testid="agent-config-target-${agentId}"]`);
  await target.click();
  await globalThis.browser.waitUntil(async () => (
    await globalThis.$('[data-testid="agent-configuration-content"]').getAttribute("data-agent-id")
  ) === agentId, { timeout: 30_000, timeoutMsg: `${agentId} never became the active Agent target.` });
}

globalThis.describe("Agent Configuration desktop UI", () => {
  let originalTheme = "minimal";

  globalThis.before(async () => {
    await waitForBootstrap();
    const settings = await invoke(({ core }) => core.invoke("get_settings"));
    originalTheme = settings.theme;
  });

  globalThis.it("uses grouped navigation and a staged CLI provider dialog in the Tauri client", async () => {
    await globalThis.browser.setWindowSize(1440, 960);
    await setTheme("minimal");
    await openSettings("agent-configurations", AGENT_PAGE);

    const targets = await globalThis.$$(`${AGENT_PAGE} [data-testid^="agent-config-target-"]`);
    assert.equal(targets.length, 6, "the desktop Agent selector did not expose all managed targets");
    assert.equal(await targets[0].getAttribute("aria-current"), "page");

    const addProfile = await globalThis.$('[data-testid="cli-config-add-profile"]');
    await addProfile.waitForClickable({ timeout: 30_000 });
    await addProfile.click();
    await globalThis.$('[data-testid="cli-config-provider-stage"]').waitForDisplayed();
    assert.equal(await globalThis.$$('[data-testid="cli-config-configuration-stage"]').then((items) => items.length), 0);

    const provider = await globalThis.$('[data-provider-id="anthropic"]');
    await provider.click();
    await globalThis.$('[data-testid="cli-config-provider-continue"]').click();
    await globalThis.$('[data-testid="cli-config-configuration-stage"]').waitForDisplayed();
    const advanced = await globalThis.$('[data-testid="cli-config-advanced-settings"]');
    assert.equal(await advanced.getAttribute("open"), null, "advanced CLI fields should start collapsed");
    await globalThis.browser.keys(["Escape"]);

    await assertNoHorizontalOverflow("minimal Agent Configuration");
    await globalThis.browser.saveScreenshot(path.join(SCREENSHOT_DIR, "agent-configuration-minimal-desktop.png"));
  });

  globalThis.it("keeps OnePiece secondary views usable in a narrow futuristic desktop window", async () => {
    await globalThis.browser.setWindowSize(520, 860);
    await setTheme("futuristic");
    await openSettings("agent-configurations", AGENT_PAGE);
    await selectAgent("onepiece");

    const providers = await globalThis.$('[data-testid="onepiece-view-providers"]');
    await providers.waitForDisplayed({ timeout: 30_000 });
    assert.equal(await globalThis.$('[data-testid="onepiece-view-tab-providers"]').getAttribute("aria-selected"), "true");

    await globalThis.$('[data-testid="onepiece-view-tab-runtime"]').click();
    await globalThis.$('[data-testid="onepiece-view-runtime"]').waitForDisplayed({ timeout: 30_000 });
    await globalThis.$('[data-testid="onepiece-view-tab-tools"]').click();
    await globalThis.$('[data-testid="onepiece-view-tools"]').waitForDisplayed({ timeout: 30_000 });

    await assertNoHorizontalOverflow("futuristic narrow OnePiece Configuration");
    await globalThis.browser.saveScreenshot(path.join(SCREENSHOT_DIR, "agent-configuration-futuristic-narrow.png"));
  });

  globalThis.it("renders Code Intelligence independently in both desktop themes", async () => {
    for (const variant of [
      { theme: "minimal", width: 1440, height: 960, suffix: "minimal-desktop" },
      { theme: "futuristic", width: 520, height: 860, suffix: "futuristic-narrow" },
    ]) {
      await globalThis.browser.setWindowSize(variant.width, variant.height);
      await setTheme(variant.theme);
      const page = await openSettings("code-intelligence", CODE_INTELLIGENCE_PAGE);
      const panels = await page.$$("section");
      assert.ok(panels.length >= 4, "Code Intelligence did not render its four desktop sections");
      const parkedAgentPages = await globalThis.$$(AGENT_PAGE);
      const parkedVisibility = [];
      for (let index = 0; index < parkedAgentPages.length; index += 1) {
        parkedVisibility.push(await parkedAgentPages[index].isDisplayed());
      }
      assert.ok(
        parkedVisibility.every((displayed) => !displayed),
        "Agent Configuration remained visible inside Code Intelligence",
      );
      await assertNoHorizontalOverflow(`${variant.theme} Code Intelligence`);
      await globalThis.browser.saveScreenshot(path.join(
        SCREENSHOT_DIR,
        `code-intelligence-${variant.suffix}.png`,
      ));
    }
  });

  globalThis.after(async () => {
    await setTheme(originalTheme).catch(() => {});
    await globalThis.browser.setWindowSize(1280, 900).catch(() => {});
    await navigate("/workspace/sessions").catch(() => {});
  });
});
