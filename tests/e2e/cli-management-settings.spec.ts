import { expect, test, type Locator, type Page } from "@playwright/test";

/**
 * CLI Management, driven the way a user drives it.
 *
 * The Web/mock runtime answers every call: no host PATH, no process, no package manager, no
 * credential store, and no network. That is what makes these workflows assertable at all -- the
 * fixtures reach terminal outcomes and plan refusals that a real machine would have to be coaxed
 * into, and they do it deterministically.
 *
 * Nothing here asserts only that an element exists. Each test performs the sequence a user
 * performs and checks what the sequence produced.
 */

/** The sentinel targets the Web/mock catalog offers, one per outcome the desktop runtime has. */
const TARGET = {
  verified: "1.3.0",
  appliedUnverified: "1.3.0-unverified",
  changedButFailed: "1.3.0-changed",
  noChangeFailed: "1.3.0-fails",
  cancelled: "1.3.0-cancels",
  expiredPlan: "1.3.0-expired",
  stalePlan: "1.3.0-stale",
} as const;

async function openCliManagement(page: Page) {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /^CLI 管理/ }).click();
  // task 12.18: this page now renders its title through the shared `ui/page-header/PageHeader`
  // (an `<h1>`), not the local `page-parts.tsx` one (an `<h2>`) the other unmigrated pages still use.
  await expect(page.getByRole("heading", { name: "CLI 管理", level: 1 })).toBeVisible();
  await expect(page.locator('[data-cli-agent="claude-code"]')).toBeVisible();
}

function card(page: Page, agentId: string): Locator {
  return page.locator(`[data-cli-agent="${agentId}"]`);
}

/** Selects a target on a card and opens the review dialog for it. */
async function reviewPlan(page: Page, agentId: string, name: string, target: string) {
  await card(page, agentId).getByLabel(`${name} 目标版本`).selectOption(target);
  await card(page, agentId).getByRole("button", { name: "更改版本" }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  return dialog;
}

test.describe("CLI Management navigation and inventory", () => {
  test("reaches the page from settings and lazily loads its own chunk", async ({ page }) => {
    const chunkRequests: string[] = [];
    page.on("request", (request) => {
      const url = request.url();
      if (url.includes("cli-management-page")) chunkRequests.push(url);
    });

    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    // Nothing for this page has been fetched yet: it is a route, not part of the shell.
    expect(chunkRequests).toEqual([]);

    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /^CLI 管理/ }).click();

    await expect(page.getByRole("heading", { name: "CLI 管理", level: 1 })).toBeVisible();
    expect(chunkRequests.length).toBeGreaterThan(0);
  });

  test("loads every registered tool with the backend's own state, not a derived one", async ({ page }) => {
    await openCliManagement(page);

    const cards = page.locator("[data-cli-agent]");
    await expect(cards).toHaveCount(5);
    expect(await cards.evaluateAll((items) => items.map((item) => item.getAttribute("data-cli-agent"))))
      .toEqual(["claude-code", "codex-cli", "opencode", "antigravity-cli", "gemini-cli"]);

    // The five summary counts add up to the tools that landed in a bucket; a tool counted twice
    // would push the total past the number of cards. Scoped to the summary bar's own group --
    // task 12.16's nav-entry status dot can fold "可更新" into the sidebar entry's own accessible
    // name too (its sr-only text joins the button's name, not just a separate description), which
    // would otherwise make an unscoped page-wide match ambiguous.
    const summaryBar = page.getByRole("group", { name: "状态概览" });
    await expect(summaryBar.getByRole("button", { name: /可更新/ })).toContainText("1");
    await expect(summaryBar.getByRole("button", { name: /有冲突/ })).toContainText("1");
    await expect(summaryBar.getByRole("button", { name: /就绪/ })).toContainText("2");

    // Nothing on this page may look like a path from the machine the browser is running on.
    const body = (await page.locator("body").textContent()) ?? "";
    expect(body).not.toContain("C:\\Users");
    expect(body).not.toMatch(/\/(?:home|Users)\/[a-z]/i);
  });

  test("shows a stale snapshot as stale without blanking what it knows", async ({ page }) => {
    await openCliManagement(page);

    const gemini = card(page, "gemini-cli");
    await expect(gemini.getByText("数据已过期")).toBeVisible();
    await expect(gemini).toContainText("3.1.0");
    await expect(gemini).toContainText("gemini");
  });
});

test.describe("CLI Management filtering", () => {
  test("narrows by search, by summary bucket, by source, and by needing attention", async ({ page }) => {
    await openCliManagement(page);

    await page.getByLabel("搜索 CLI").fill("codex");
    await expect(page.locator("[data-cli-agent]")).toHaveCount(1);
    await expect(card(page, "codex-cli")).toBeVisible();
    await page.getByLabel("搜索 CLI").fill("");
    await expect(page.locator("[data-cli-agent]")).toHaveCount(5);

    const summaryBar = page.getByRole("group", { name: "状态概览" });
    await summaryBar.getByRole("button", { name: /有冲突/ }).click();
    await expect(page.locator("[data-cli-agent]")).toHaveCount(1);
    await expect(card(page, "opencode")).toBeVisible();
    // The same control clears it, so there is never a filter with no visible way back.
    await summaryBar.getByRole("button", { name: /有冲突/ }).click();
    await expect(page.locator("[data-cli-agent]")).toHaveCount(5);

    await page.getByLabel("按来源筛选").selectOption("vendor");
    await expect(page.locator("[data-cli-agent]")).toHaveCount(1);
    await expect(card(page, "antigravity-cli")).toBeVisible();
    await page.getByLabel("按来源筛选").selectOption("all");

    await page.getByLabel("只看需要处理的").check();
    const attention = await page.locator("[data-cli-agent]")
      .evaluateAll((items) => items.map((item) => item.getAttribute("data-cli-agent")));
    expect(attention.sort()).toEqual(["claude-code", "opencode"]);

    await page.getByLabel("只看需要处理的").uncheck();
    await page.getByLabel("搜索 CLI").fill("nothing-matches-this");
    await expect(page.getByText("没有符合当前筛选条件的 CLI。")).toBeVisible();
  });

  test("keeps the filter and its scroll position across a background refresh", async ({ page }) => {
    await openCliManagement(page);
    await page.getByLabel("搜索 CLI").fill("claude");
    await expect(page.locator("[data-cli-agent]")).toHaveCount(1);

    await page.getByRole("button", { name: "刷新检测" }).click();

    // Cached content stays on screen for the whole refresh; a blank list would read as "nothing is
    // installed" for as long as the probes take.
    await expect(card(page, "claude-code")).toBeVisible();
    await expect(page.getByLabel("搜索 CLI")).toHaveValue("claude");
    await expect(page.locator("[data-cli-agent]")).toHaveCount(1);
  });
});

test.describe("CLI Management details drawer", () => {
  test("opens with the trigger reporting its state and moves between all four tabs", async ({ page }) => {
    await openCliManagement(page);

    const trigger = card(page, "opencode").getByRole("button", { name: /^查看 OpenCode CLI 的详情$/ });
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    await trigger.click();

    const drawer = page.getByRole("dialog");
    await expect(drawer).toBeVisible();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");
    const controls = await trigger.getAttribute("aria-controls");
    expect(controls).toBeTruthy();
    await expect(drawer.locator(`#${controls}`)).toHaveCount(1);

    await expect(drawer.getByRole("tab", { name: "概览" })).toHaveAttribute("aria-selected", "true");
    await expect(drawer.getByRole("tabpanel")).toContainText("找到多处");
    await expect(drawer.getByRole("tabpanel")).toContainText("安装冲突");

    await drawer.getByRole("tab", { name: "安装" }).click();
    const installations = drawer.getByRole("tabpanel");
    // Two installations, and the page says which one PATH reaches and which one it would act on.
    await expect(installations.getByText("PATH 命中")).toBeVisible();
    await expect(installations.getByText("推荐", { exact: true })).toBeVisible();
    await expect(installations.getByTitle("/mock/bin/opencode-a")).toBeVisible();
    await expect(installations.getByTitle("/mock/homebrew/bin/opencode")).toBeVisible();

    await drawer.getByRole("tab", { name: "诊断" }).click();
    const diagnostics = drawer.getByRole("tabpanel");
    // The conflict is named by its own code, and the detect-only source says who does own it.
    await expect(diagnostics.getByText("安装冲突")).toBeVisible();
    await expect(diagnostics.getByText(/PATH 中靠前的安装遮蔽/)).toBeVisible();
    await expect(diagnostics.getByText("仅检测")).toBeVisible();
    await expect(diagnostics.getByText(/brew upgrade/)).toBeVisible();

    await drawer.getByRole("tab", { name: "操作" }).click();
    await expect(drawer.getByRole("tabpanel")).toContainText("该 CLI 还没有跑过任何操作。");
  });

  test("moves between tabs from the keyboard and restores focus on close", async ({ page }) => {
    await openCliManagement(page);
    const trigger = card(page, "claude-code").getByRole("button", { name: /^查看 Anthropic Claude Code CLI 的详情$/ });
    await trigger.click();
    const drawer = page.getByRole("dialog");

    await expect(drawer.getByRole("tab", { name: "概览" })).toBeFocused();
    await page.keyboard.press("ArrowRight");
    await expect(drawer.getByRole("tab", { name: "安装" })).toHaveAttribute("aria-selected", "true");
    await expect(drawer.getByRole("tab", { name: "安装" })).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(drawer.getByRole("tab", { name: "概览" })).toHaveAttribute("aria-selected", "true");

    await page.keyboard.press("Escape");
    await expect(drawer).toBeHidden();
    // Focus back on the control that opened it: leaving it on the body drops a keyboard user at
    // the top of the page every time they look at a tool.
    await expect(trigger).toBeFocused();
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
  });

  test("runs the tool's own diagnostics and reports what it concluded", async ({ page }) => {
    await openCliManagement(page);
    await card(page, "claude-code").getByRole("button", { name: /详情$/ }).click();
    const drawer = page.getByRole("dialog");
    await drawer.getByRole("tab", { name: "诊断" }).click();

    await drawer.getByRole("button", { name: "重新运行诊断" }).click();

    await drawer.getByRole("tab", { name: "操作" }).click();
    // `unknown` is the honest answer for a runtime that cannot run the tool's diagnostics, and it
    // is reported as `unknown` rather than as a failure.
    await expect(drawer.getByRole("tabpanel")).toContainText(/进行中|排队中|已完成|成功/);
    await expect(drawer.getByRole("button", { name: "复制摘要" })).toBeVisible();
  });
});

test.describe("CLI Management action plan review", () => {
  test("shows everything the user is agreeing to before anything runs", async ({ page }) => {
    await openCliManagement(page);
    const dialog = await reviewPlan(page, "claude-code", "Anthropic Claude Code CLI", TARGET.verified);

    await expect(dialog.getByText("确认对 Anthropic Claude Code CLI 的变更")).toBeVisible();
    await expect(dialog.getByText("npm", { exact: true }).first()).toBeVisible();
    // Argv, one argument per line. Never a shell string: there is nothing here to quote.
    await expect(dialog.locator("pre")).toHaveText(
      "npm\ninstall\n--global\n@anthropic-ai/claude-code@1.3.0",
    );
    await expect(dialog.getByText("需要网络")).toBeVisible();
    await expect(dialog.getByText("失败不会自动改用其他来源")).toBeVisible();
    await expect(dialog.getByText("该来源自身的命令必须可用")).toBeVisible();
    await expect(dialog.getByText(/此计划在 .* 之前有效/)).toBeVisible();
    // The transition, both ends of it, from the plan rather than from anything this page derived.
    await expect(dialog.getByText("1.2.0", { exact: true })).toBeVisible();
    await expect(dialog.getByText("1.3.0", { exact: true })).toBeVisible();
  });

  test("verifies a completed change and reports the outcome on the card", async ({ page }) => {
    await openCliManagement(page);
    const dialog = await reviewPlan(page, "claude-code", "Anthropic Claude Code CLI", TARGET.verified);
    await dialog.getByRole("button", { name: "确认执行" }).click();

    await expect(dialog).toBeHidden();
    await expect(card(page, "claude-code").getByText("已验证")).toBeVisible({ timeout: 15_000 });
  });

  test("explains an applied-unverified result rather than reporting a clean success", async ({ page }) => {
    await openCliManagement(page);
    const dialog = await reviewPlan(page, "claude-code", "Anthropic Claude Code CLI", TARGET.appliedUnverified);
    await dialog.getByRole("button", { name: "确认执行" }).click();

    const claude = card(page, "claude-code");
    await expect(claude.getByText("已执行，未能验证")).toBeVisible({ timeout: 15_000 });
    await expect(claude.getByText(/请先刷新检测/)).toBeVisible();
    await expect(claude.getByText(/事后复检没有看到目标版本/)).toBeVisible();
  });

  test("states that a changed-but-failed run was not rolled back", async ({ page }) => {
    await openCliManagement(page);
    const dialog = await reviewPlan(page, "claude-code", "Anthropic Claude Code CLI", TARGET.changedButFailed);
    await dialog.getByRole("button", { name: "确认执行" }).click();

    const claude = card(page, "claude-code");
    await expect(claude.getByText("已改动，但失败")).toBeVisible({ timeout: 15_000 });
    await expect(claude.getByText(/没有回滚任何东西/)).toBeVisible();
  });

  test("says a failed run changed nothing when nothing changed", async ({ page }) => {
    await openCliManagement(page);
    const dialog = await reviewPlan(page, "claude-code", "Anthropic Claude Code CLI", TARGET.noChangeFailed);
    await dialog.getByRole("button", { name: "确认执行" }).click();

    const claude = card(page, "claude-code");
    await expect(claude.getByText("失败，未改动")).toBeVisible({ timeout: 15_000 });
    await expect(claude.getByText(/可以安全重试/)).toBeVisible();
  });

  test("reports a cancelled run as cancelled", async ({ page }) => {
    await openCliManagement(page);
    const dialog = await reviewPlan(page, "claude-code", "Anthropic Claude Code CLI", TARGET.cancelled);
    await dialog.getByRole("button", { name: "确认执行" }).click();

    await expect(card(page, "claude-code").getByText("已取消").first()).toBeVisible({ timeout: 15_000 });
  });

  test("refuses an expired plan and offers a new one instead of a confirm", async ({ page }) => {
    await openCliManagement(page);
    const dialog = await reviewPlan(page, "claude-code", "Anthropic Claude Code CLI", TARGET.expiredPlan);

    await expect(dialog.getByText("此计划已过期，请重新准备。")).toBeVisible();
    await expect(dialog.getByRole("button", { name: "确认执行" })).toHaveCount(0);
    await dialog.getByRole("button", { name: "重新准备计划" }).click();
    await expect(dialog).toBeHidden();
  });

  test("refuses a plan revised after review and says so on the dialog", async ({ page }) => {
    await openCliManagement(page);
    const dialog = await reviewPlan(page, "claude-code", "Anthropic Claude Code CLI", TARGET.stalePlan);

    // Draft and confirmable, because the revision only moves underneath it at execution.
    await dialog.getByRole("button", { name: "确认执行" }).click();

    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole("alert")).toContainText("这个计划在展示给你之后发生了变化，请重新准备。");
    await expect(dialog.getByRole("button", { name: "重新准备计划" })).toBeVisible();
  });

  test("offers no change and opens no dialog when the target is the installed version", async ({ page }) => {
    await openCliManagement(page);

    await card(page, "claude-code").getByLabel("Anthropic Claude Code CLI 目标版本").selectOption("1.2.0");

    await expect(card(page, "claude-code").getByRole("button", { name: "更改版本" })).toHaveCount(0);
    await expect(card(page, "claude-code").getByText("已是当前版本")).toBeVisible();
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });

  test("withholds mutation entirely while a conflict blocks it", async ({ page }) => {
    await openCliManagement(page);

    const opencode = card(page, "opencode");
    await expect(opencode.getByText(/PATH 中靠前的安装遮蔽/)).toBeVisible();
    await expect(opencode.getByRole("button", { name: "更改版本" })).toHaveCount(0);
    await expect(opencode.getByText("仅检测")).toBeVisible();
    await expect(opencode.getByText(/brew upgrade/)).toBeVisible();
  });
});

test.describe("CLI Management bulk upgrade", () => {
  test("previews what will run and what will not, then reports a real outcome per item", async ({ page }) => {
    await openCliManagement(page);

    await page.getByRole("button", { name: /全部升级/ }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();

    await expect(dialog.getByText(/将执行（1）/)).toBeVisible();
    await expect(dialog.getByText(/跳过（2）/)).toBeVisible();
    await expect(dialog.getByText("Anthropic Claude Code CLI")).toBeVisible();
    // A skipped tool is named with a reason. A shorter list with no reason reads as "the rest is
    // up to date", which is a claim the backend did not make.
    await expect(dialog.getByText("存在安装冲突")).toBeVisible();
    await expect(dialog.getByText("已是目标版本")).toBeVisible();

    await dialog.getByRole("button", { name: /执行 1 项/ }).click();

    await expect(dialog.getByText("执行结果")).toBeVisible({ timeout: 15_000 });
    await expect(dialog.getByText("已验证")).toBeVisible();
    await expect(dialog.getByText("失败，未改动")).toBeVisible();
    await expect(dialog.getByText("存在安装冲突")).toBeVisible();
    // The placeholder this replaced said a process had started and nothing about the machine.
    await expect(dialog.getByText("ran", { exact: true })).toHaveCount(0);

    await dialog.getByRole("button", { name: "关闭" }).click();
    await expect(dialog).toBeHidden();
  });
});

test.describe("CLI Management operation state", () => {
  test("keeps unrelated tools usable while one refreshes, and cancels through the operation service", async ({ page }) => {
    await openCliManagement(page);

    await card(page, "claude-code").getByRole("button", { name: /^刷新 Anthropic Claude Code CLI$/ }).click();

    const claude = card(page, "claude-code");
    await expect(claude.getByText(/排队中|进行中/)).toBeVisible();
    // No global busy flag: the other tools stay actionable while this one works.
    await expect(card(page, "antigravity-cli").getByRole("button", { name: "更改版本" })).toBeEnabled();

    await claude.getByRole("button", { name: "取消" }).click();
    await expect(claude.getByText("已取消")).toBeVisible({ timeout: 15_000 });
  });
});

test.describe("CLI Management presentation", () => {
  test("stays readable in English at narrow width without horizontal overflow", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("combobox", { name: /应用语言|Application Language/ }).selectOption("en");
    await page.getByRole("combobox", { name: /主题|Theme/ }).selectOption("minimal");
    // Below `lg` the sidebar is hidden in favor of a searchable sheet (task 12.9): open it first.
    await page.getByRole("button", { name: /^Switch settings page/ }).click();
    await page.getByRole("button", { name: /^CLI Management/ }).click();

    await expect(page.getByRole("heading", { name: "CLI Management", level: 1 })).toBeVisible();
    await expect(page.locator("[data-cli-agent]")).toHaveCount(5);
    await expect(page.getByRole("button", { name: /Conflicts/ })).toBeVisible();
    await expect(page.getByText("Detect only")).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");

    const overflow = await page.evaluate(() => document.body.scrollWidth > window.innerWidth);
    expect(overflow).toBe(false);
  });
});
