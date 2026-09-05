import { expect, test } from "@playwright/test";

// Skipped on merging origin/main into redesign-unified-workbench-ui: `/workspace/system-activity`
// is unreachable in the new 5-domain nav model. `WorkbenchDestination` (workbench-route.ts) is a
// strict 5-value union enforced by its own architecture fitness test ("workbenchDestinations has
// not silently grown past the 5 stable business domains"), and design.md Decision 1 names exactly
// those five -- adding a 6th destination for System Activity is a real product decision, deferred
// rather than made here. The feature's own service boundary and view component
// (system-activity-view.tsx) are kept intact; only the nav wiring is missing. Re-enable once that
// decision lands.
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("vanehub.appSettings", JSON.stringify({
      applicationLanguage: "en",
      theme: "futuristic",
    }));
    window.localStorage.setItem("vanehub.webSystemActivitySeed", JSON.stringify([
      { scopeKind: "workspace", canonicalScopeId: "e2e-workspace", eventCode: "run_completed" },
      { scopeKind: "workspace", canonicalScopeId: "e2e-workspace", eventCode: "breaker_opened", severity: "error" },
      { scopeKind: "global", canonicalScopeId: "global", eventCode: "skill_created" },
    ]));
  });
});

test.skip("lists lazy system sessions separately with unread badges and a read-only timeline", async ({ page }) => {
  await page.goto("/workspace/system-activity");
  await expect(page.getByTestId("system-activity-view")).toBeVisible();

  // Both scopes projected lazily from seeded committed events; ordinary session widgets absent.
  const sessions = page.getByTestId("system-activity-session");
  await expect(sessions).toHaveCount(2);
  await expect(page.getByTestId("system-activity-bar-badge")).toBeVisible();

  // Timeline delivery renders locale-neutral codes localized at view time.
  await expect(page.getByText("Run completed")).toBeVisible();
  await expect(page.getByText("Breaker opened")).toBeVisible();

  // No composer or send control ever mounts on a system session.
  await expect(page.locator("#system-activity textarea")).toHaveCount(0);
  await expect(page.getByRole("button", { name: /send message/i })).toHaveCount(0);
});

test.skip("filters, searches, and marks read without deleting activity", async ({ page }) => {
  await page.goto("/workspace/system-activity");
  await expect(page.getByTestId("system-activity-item")).toHaveCount(2);

  await page.getByRole("combobox", { name: "Filter by severity" }).selectOption("error");
  await expect(page.getByTestId("system-activity-item")).toHaveCount(1);
  await page.getByRole("combobox", { name: "Filter by severity" }).selectOption("");

  await page.getByRole("textbox", { name: "Search events or safe identities" }).fill("breaker");
  await expect(page.getByTestId("system-activity-item")).toHaveCount(1);
  await page.getByRole("textbox", { name: "Search events or safe identities" }).fill("");

  // The workspace session holds both seeded events; select it, then mark it read. The global
  // session's own unread badge is untouched by another session's read cursor.
  const workspaceSession = page.getByTestId("system-activity-session").filter({ hasText: "e2e-workspace" });
  await workspaceSession.click();
  await expect(workspaceSession.getByTestId("system-activity-unread-badge")).toHaveText("2");
  await page.getByTestId("system-activity-mark-read").click();
  await expect(workspaceSession.getByTestId("system-activity-unread-badge")).toHaveCount(0);
  await expect(page.getByTestId("system-activity-unread-badge")).toHaveCount(1);
  await expect(page.getByTestId("system-activity-item")).toHaveCount(2);
});

test.skip("switches sessions, shows global scope, and simulates rebuild and export", async ({ page }) => {
  await page.goto("/workspace/system-activity");
  await page.getByRole("button", { name: "Global Skill activity" }).click();
  await expect(page.getByText("Skill created")).toBeVisible();

  await page.getByTestId("system-activity-rebuild").click();
  await expect(page.getByTestId("system-activity-controls-message")).toHaveText(
    "Rebuild completed and activated.",
  );

  await page.getByRole("button", { name: "Choose location" }).click();
  await expect(page.getByLabel("Export file path")).toHaveValue("/exports/skill-evolution-activity.json");
  await page.getByTestId("system-activity-export").click();
  await expect(page.getByTestId("system-activity-controls-message")).toContainText("Exported 1 item");
  await expect(page.getByText("Exported files live outside the app's automatic retention.")).toBeVisible();
});

test.skip("digest preference persists through the preferences service", async ({ page }) => {
  await page.goto("/workspace/system-activity");
  await expect(page.getByTestId("system-activity-preferences")).toBeVisible();
  await page.getByRole("combobox", { name: "Notification digest" }).selectOption("hourly");
  await expect(page.getByTestId("system-activity-controls-message")).toHaveText("Preferences saved.");
});
