import { expect, test } from "@playwright/test";

test.describe("Skills management", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en" }));
    });
  });

  test("manages global Skills by dynamic CLI Agent without scope controls", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/settings");
    await page.getByRole("button", { name: "Skills", exact: true }).click();

    await expect(page.getByRole("heading", { name: "Skills", exact: true, level: 2 })).toBeVisible();
    await expect(page.getByRole("button", { name: /^All Skills/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /^Unassigned/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /^Global$/ })).toHaveCount(0);
    await expect(page.getByRole("button", { name: /^Workspace$/ })).toHaveCount(0);

    const systemSkill = page.locator('article[data-skill-id="tdd-discipline"]');
    await expect(systemSkill).toContainText("System");
    await expect(systemSkill).toContainText("Role");
    await expect(systemSkill).toContainText("Read-only");
    await expect(systemSkill).not.toContainText("On demand");
    await expect(systemSkill.getByRole("button", { name: "Edit Skill" })).toHaveCount(0);
    await expect(systemSkill.getByRole("button", { name: "Delete Skill" })).toHaveCount(0);

    const webUtility = page.locator('article[data-skill-id="code-security-scan"]');
    await expect(webUtility).toContainText("Utility");
    await expect(webUtility).toContainText("Delegation unavailable");
    await webUtility.getByRole("button", { name: /View details for/ }).click();
    const utilityInspector = page.getByRole("complementary", { name: /details/ });
    await expect(utilityInspector).toContainText("Utility delegation is unavailable in this runtime");
    await utilityInspector.getByRole("button", { name: "Close Skill details" }).click();

    const migratedOverride = page.locator('article[data-skill-id="readme-generation"]');
    await expect(migratedOverride).toHaveCount(1);
    const detailsToggle = migratedOverride.getByRole("button", { name: /View details for/ });
    await expect(detailsToggle).toHaveAttribute("aria-expanded", "false");
    await detailsToggle.click();
    await expect(detailsToggle).toHaveAttribute("aria-expanded", "true");
    const inspector = page.getByRole("complementary", { name: /details/ });
    await expect(inspector).toContainText("Project > User > Registry > System");
    await expect(inspector.getByRole("heading", { name: "Definition precedence" })).toBeVisible();
    await expect(inspector.getByRole("heading", { name: "Evolution evidence" })).toBeVisible();
    await expect(inspector).toContainText("Target selection and Skill modification are not active");
    await inspector.getByRole("button", { name: "Inspect lineage" }).click();
    await expect(inspector.getByRole("region", { name: "Candidate seed lineage" })).toBeVisible();
    const purgeEvidence = inspector.getByRole("button", { name: "Purge this Skill's evidence" });
    await purgeEvidence.click();
    const purgeDialog = page.getByRole("dialog", { name: "Purge evolution evidence?" });
    await expect(purgeDialog).toContainText("Source conversations, traces, logs, usage, Skills, and Overlays remain unchanged");
    await page.keyboard.press("Escape");
    await expect(purgeEvidence).toBeFocused();
    await expect(inspector.locator('[data-definition-state="effective"]')).toHaveCount(1);
    await expect(inspector.locator('[data-definition-state="shadowed"]')).toHaveCount(1);
    const inspectorBox = await inspector.boundingBox();
    const selectedRowBox = await migratedOverride.boundingBox();
    expect(inspectorBox).not.toBeNull();
    expect(selectedRowBox).not.toBeNull();
    expect(inspectorBox!.x).toBeGreaterThan(selectedRowBox!.x);
    await inspector.getByRole("button", { name: "Close Skill details" }).click();
    await expect(detailsToggle).toBeFocused();

    const previewButton = systemSkill.getByRole("button", { name: "Preview Skill" });
    await previewButton.click();
    const previewDialog = page.getByRole("dialog");
    await expect(previewDialog).toContainText("indexed resources");
    await expect(previewDialog).toContainText("read-only");
    await page.keyboard.press("Escape");
    await expect(previewDialog).toBeHidden();
    await expect(previewButton).toBeFocused();

    await page.getByRole("button", { name: /Codex CLI/ }).click();
    const board = page.getByTestId("skill-selection-board");
    const assigned = board.locator('[data-skill-group="assigned"]');
    const available = board.locator('[data-skill-group="available"]');
    await expect(assigned.getByRole("heading", { name: "Assigned" })).toBeVisible();
    await expect(available.getByRole("heading", { name: "Available" })).toBeVisible();
    const assignedBox = await assigned.boundingBox();
    const availableBox = await available.boundingBox();
    expect(assignedBox).not.toBeNull();
    expect(availableBox).not.toBeNull();
    expect(availableBox!.x).toBeGreaterThan(assignedBox!.x + 100);
    expect(Math.abs(availableBox!.y - assignedBox!.y)).toBeLessThan(8);
    await expect(page.getByRole("checkbox", { name: "Enabled" })).toHaveCount(0);
    await expect(page.getByRole("checkbox", { name: /Assign to|Unassign from/ })).toHaveCount(0);
    await expect(page.getByRole("button", { name: /Edit Skill|Delete Skill/ })).toHaveCount(0);
    await expect(page.getByText("Globally enabled").first()).toBeVisible();

    const assignmentAction = page.getByRole("button", { name: /Assign to Codex CLI|Unassign from Codex CLI/ }).first();
    const assignmentRow = assignmentAction.locator("xpath=ancestor::article");
    await expect(assignmentRow.getByRole("button", { name: /View details for/ })).toBeVisible();
    await expect(assignmentRow.getByRole("button", { name: "Preview Skill" })).toBeVisible();
    const actionName = await assignmentAction.getAttribute("aria-label");
    const skillId = await assignmentAction.locator("xpath=ancestor::article").getAttribute("data-skill-id");
    await assignmentAction.focus();
    await expect(assignmentAction).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.locator(`article[data-skill-id="${skillId}"] button[aria-label="${actionName}"]`)).toHaveCount(0);

    const advanced = page.locator("details").filter({ hasText: "Agent mount paths" });
    await expect(advanced).not.toHaveAttribute("open", "");
    await advanced.locator("summary").click();
    await expect(advanced.locator("input")).toHaveValue(/\.codex[/\\]skills/);
  });

  test("supports inventory filters and focus-safe application dialogs", async ({ page }) => {
    await page.setViewportSize({ width: 760, height: 820 });
    await page.goto("/settings");
    await page.getByRole("button", { name: "Skills", exact: true }).click();
    await page.getByPlaceholder("Search Skills...").fill("readme");
    await expect(page.getByText(/items$/).first()).toBeVisible();
    await page.getByPlaceholder("Search Skills...").fill("");

    const localSearch = page.getByPlaceholder("Search by id, name, category, trigger, or source");
    await localSearch.fill("readme");
    await expect(page.getByText("1 items")).toBeVisible();
    await localSearch.fill("");

    const codexAgent = page.getByRole("button", { name: /Codex CLI/ });
    await codexAgent.locator("span.truncate").evaluate((label) => {
      label.textContent = "OpenAI Codex CLI with an exceptionally long registered Agent display name";
    });
    await expect(codexAgent.locator("span.truncate")).toHaveCSS("text-overflow", "ellipsis");
    await codexAgent.click();
    const board = page.getByTestId("skill-selection-board");
    const assigned = board.locator('[data-skill-group="assigned"]');
    const available = board.locator('[data-skill-group="available"]');
    const assignedBox = await assigned.boundingBox();
    const availableBox = await available.boundingBox();
    expect(assignedBox).not.toBeNull();
    expect(availableBox).not.toBeNull();
    expect(availableBox!.y).toBeGreaterThan(assignedBox!.y + assignedBox!.height - 2);
    expect(Math.abs(availableBox!.x - assignedBox!.x)).toBeLessThan(8);

    await page.getByRole("button", { name: /^All Skills/ }).click();
    await page.setViewportSize({ width: 375, height: 780 });
    const narrowDetailsButton = page.locator("article[data-skill-id]").first().getByRole("button", { name: /View details for/ });
    await narrowDetailsButton.click();
    const detailsDialog = page.getByRole("dialog", { name: /details/ });
    await expect(detailsDialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(detailsDialog).toBeHidden();
    await expect(narrowDetailsButton).toBeFocused();
    const hasHorizontalOverflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
    expect(hasHorizontalOverflow).toBe(false);

    const restoreButton = page.getByRole("button", { name: /Restore Built-in/ });
    await restoreButton.click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.locator("select")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(restoreButton).toBeFocused();
  });

  test("inspects exact tool governance while Web keeps native actions unavailable", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/settings");
    await page.getByRole("button", { name: "Skills", exact: true }).click();
    const skill = page.locator('article[data-skill-id="code-review"]');
    await skill.getByRole("button", { name: /View details for/ }).click();
    const inspector = page.getByRole("complementary", { name: /details/ });
    await inspector.getByRole("tab", { name: "Tools" }).click();
    const tool = inspector.locator('article[data-tool-revision]');
    await expect(tool).toContainText("inspect-diff");
    await expect(tool).toContainText("Inspection only; Web cannot execute local tools");
    await expect(tool).toContainText("Redacted validation and runtime diagnostics");
    await expect(tool.getByRole("button", { name: "Validate revision" })).toBeDisabled();
    await expect(tool.getByRole("button", { name: "Trust revision" })).toBeDisabled();
    await expect(tool.getByRole("button", { name: "Enable separately" })).toBeDisabled();
    await expect(tool.getByRole("button", { name: "Quarantine" })).toBeDisabled();
    await expect(tool).toContainText("Inspection data is read-only");
    await expect(tool).toHaveAttribute("data-tool-revision", /^[a-f0-9]{64}$/);
  });

  test("governs a mocked native exact revision through trust, enablement, quarantine, and recovery", async ({ page }) => {
    await page.addInitScript(() => {
      const callbacks = new Map<number, (value: unknown) => void>();
      let callbackId = 0;
      const skill = {
        id: "native-tools", scope: "global", workspacePath: null, source: "user", enabled: true,
        skillDir: "/skills/native-tools", skillMdPath: "/skills/native-tools/SKILL.md", contentHash: "base-native",
        metadata: { id: "native-tools", name: "Native Tool Skill", description: "Native governance fixture", category: "testing", version: "1.0.0", triggers: [] },
        boundAgentIds: [], bindings: [], createdAt: "now", updatedAt: "now", layer: "user", origin: "created", trust: "trusted",
        availability: "available", immutable: false, shadowedDefinitions: [], usage: { viewCount: 0, useCount: 0, lastViewedAt: null, lastUsedAt: null, revisionWitness: null },
      };
      let tool = {
        skillId: skill.id, toolId: "inspect-native", canonicalId: "skill__native-tools__inspect-native__ffffffffffff", revision: "f".repeat(64),
        sourceScope: "global", implementationKind: "declarative", baseRevision: "base-native", manifestHash: `sha256:${"a".repeat(64)}`,
        implementationHash: `sha256:${"b".repeat(64)}`, capabilityDigest: "read-workspace",
        capabilityDiff: { currentDigest: "read-workspace", added: ["filesystem.read"], removed: [], changed: true },
        validation: "valid", trusted: false, enabled: false, quarantined: false, consecutiveFailures: 0, diagnostics: [], runtimeSupport: "supported",
        enforcementStrength: "bounded-native-io", createdAt: "now", updatedAt: "now",
      };
      const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
      Object.assign(window, { __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => undefined }, __TAURI_INTERNALS__: {
        metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main", windowLabel: "main" } },
        transformCallback: (callback: (value: unknown) => void) => { const id = ++callbackId; callbacks.set(id, callback); return id; },
        unregisterCallback: (id: number) => callbacks.delete(id),
        invoke: async (command: string) => {
          if (command === "plugin:event|listen") return 1;
          if (command === "plugin:event|unlisten") return null;
          if (command === "get_settings") return { applicationLanguage: "en", fontSize: "14px", theme: "futuristic", defaultFolderPath: "", logDirectory: "", networkProxyUrl: "", networkProxyBypass: "", launchOnStartup: false, defaultPolicyTemplate: "standard", loggingPolicy: { retentionDays: 30, archiveEnabled: true, redactionEnabled: true, levels: ["error", "warn", "info"], canOpenDirectory: true }, customInstructionsAboutUser: "", customInstructionsStyleRules: "", customInstructionsEnabled: false, memoryEnabled: true, memoryToolAssistedChatsEnabled: false, automaticContextCompactionEnabled: true, contextQualityRetentionDays: 30 };
          if (command === "get_skill_overview") return { skills: [skill], stats: { total: 1, enabled: 1, mounted: 0 }, agents: [], mountPaths: [], apiAgentBindings: {}, restoreCandidates: [], drift: { scope: "global", workspacePath: null, issues: [], driftHash: "clean" } };
          if (command === "list_skill_tools") return [clone(tool)];
          if (command === "load_skill") return { status: "refused", refusal: { reason: "unsupported" } };
          if (command === "get_skill_overlay_detail") throw new Error("overlay unavailable in fixture");
          if (command === "query_skill_evolution_evidence") throw new Error("evidence unavailable in fixture");
          if (command === "set_skill_tool_trust") { tool = { ...tool, trusted: true, diagnostics: [{ severity: "info", code: "approval-provenance", detail: "settings-user · exact revision" }] }; return clone(tool); }
          if (command === "set_skill_tool_enabled") { tool = { ...tool, enabled: true }; return clone(tool); }
          if (command === "quarantine_skill_tool") { tool = { ...tool, enabled: false, quarantined: true, quarantineReason: "manual-security-review" }; return clone(tool); }
          if (command === "recover_skill_tool") { tool = { ...tool, revision: "e".repeat(64), canonicalId: "skill__native-tools__inspect-native__eeeeeeeeeeee", trusted: false, enabled: false, quarantined: false, quarantineReason: undefined, validation: "pending", diagnostics: [] }; return clone(tool); }
          throw new Error(`unsupported fixture command: ${command}`);
        },
      } });
    });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/settings");
    await page.getByRole("button", { name: "Skills", exact: true }).click();
    const skill = page.locator('article[data-skill-id="native-tools"]');
    await skill.getByRole("button", { name: /View details for/ }).click();
    const inspector = page.getByRole("complementary", { name: /details/ });
    await inspector.getByRole("tab", { name: "Tools" }).click();
    let tool = inspector.locator('article[data-tool-revision]');
    await tool.getByRole("button", { name: "Trust revision" }).click();
    await page.getByRole("dialog", { name: "Trust exact tool revision" }).getByRole("button", { name: "Trust this revision" }).click();
    await expect(tool).toContainText("approval-provenance");
    await expect(tool).toContainText("settings-user · exact revision");
    await tool.getByRole("button", { name: "Enable separately" }).click();
    await expect(tool).toContainText("Enabled");
    await tool.getByRole("button", { name: "Quarantine" }).click();
    await page.getByRole("dialog", { name: "Quarantine exact revision" }).getByRole("button", { name: "Quarantine revision" }).click();
    await expect(tool).toContainText("manual-security-review");
    await tool.getByRole("button", { name: "Recover after review" }).click();
    tool = inspector.locator('article[data-tool-revision]');
    await expect(tool).toHaveAttribute("data-tool-revision", "e".repeat(64));
    await expect(tool).toContainText("Validation pending");
    await expect(tool).toContainText("Untrusted");
  });

  test("exposes Effective, Global, and Project Skill views in session information", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "New", exact: true }).click();
    const project = page.getByPlaceholder(/code.*project/);
    await project.fill("D:\\example-workspace");
    await project.press("Tab");
    const create = page.getByRole("button", { name: "Create", exact: true });
    await expect(create).toBeEnabled();
    await create.click();
    await page.getByRole("tab", { name: "Skill", exact: true }).click();
    await expect(page.getByRole("tab", { name: "Effective" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Global" })).toBeVisible();
    await page.getByRole("tab", { name: "Project" }).click();
    await expect(page.getByText("Project Skill context")).toBeVisible();
    await expect(page.getByRole("button", { name: "Create Skill" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Import Skill" })).toBeVisible();
  });
});
