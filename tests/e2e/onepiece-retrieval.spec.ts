import { expect, test, type Locator, type Page } from "@playwright/test";

async function openAgentConfigurations(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /^(Agent 配置|Agent Configurations)$/ }).click();
  await page.getByRole("tab", { name: /OnePiece/ }).click();
  await expect(page.getByRole("tabpanel", { name: "OnePiece" }).getByRole("heading", { name: /^(API 提供商|API providers)$/i })).toBeVisible();
}

// The count in each status box (`onepiece-retrieval-section.tsx`) is an unlabelled `<p>` sibling
// right after its label `<p>` — there is no ARIA relation between the two, so the label text is
// the only reliable anchor to the number next to it.
function statusValue(section: Locator, label: string) {
  return section.getByText(label, { exact: true }).locator("xpath=following-sibling::p[1]");
}

test.describe("OnePiece retrieval configuration", () => {
  test("configuring an embedding source and triggering indexing requeues indexed/failed rows as pending", async ({ page }) => {
    await openAgentConfigurations(page);
    const onepiecePanel = page.getByRole("tabpanel", { name: "OnePiece" });

    // Retrieval only accepts openai-compatible profiles as an embedding source (Anthropic has no
    // embeddings API) — create one the same way onepiece-agent.spec.ts does for OpenRouter.
    await onepiecePanel.getByRole("button", { name: "新增配置" }).first().click();
    const addDialog = page.getByRole("dialog", { name: "新增 OnePiece 配置" });
    await addDialog.getByRole("button", { name: /OpenRouter/ }).click();
    await addDialog.getByLabel("配置名称").fill("检索 Embedding 源");
    await addDialog.getByLabel("模型", { exact: true }).selectOption("anthropic/claude-sonnet-4.6");
    await addDialog.getByLabel("API 密钥").fill("not-persisted-playwright-secret");
    await addDialog.getByRole("button", { name: "保存 OnePiece" }).click();
    await expect(onepiecePanel.getByRole("heading", { name: "检索 Embedding 源" })).toBeVisible();

    const retrievalSection = onepiecePanel.getByRole("region", { name: "检索索引配置" });
    await expect(retrievalSection).toBeVisible();

    // Baseline: the mock seeds a self-consistent global status (web-agent-client.ts
    // `seededWebRetrievalIndexStatus`): indexed=12, pending=3, failed=2. Status and rebuild are
    // global rather than per-agent, matching the configuration singleton next to them.
    await expect(statusValue(retrievalSection, "已索引")).toHaveText("12");
    await expect(statusValue(retrievalSection, "待索引")).toHaveText("3");
    await expect(statusValue(retrievalSection, "失败")).toHaveText("2");

    await retrievalSection.getByRole("combobox", { name: "Embedding 来源" }).selectOption({ label: "检索 Embedding 源" });
    await retrievalSection.getByRole("combobox", { name: "Embedding 模型" }).selectOption("text-embedding-3-small");
    await retrievalSection.getByRole("button", { name: "保存检索配置" }).click();
    await expect(retrievalSection.getByRole("status")).toHaveText("检索配置已保存。");

    // Trigger indexing. This "重建索引" action is the only indexing trigger the UI exposes. In the
    // real backend (contexts/retrieval/api.rs `rebuild`) it synchronously requeues indexed/failed
    // rows as pending, and a separate async background worker (Task 8/12) then embeds them and
    // moves them on to `indexed`. The mock (web-agent-client.ts `rebuildRetrievalIndex`) mirrors
    // only the synchronous requeue half of that contract — it has no simulated worker, so nothing
    // in the Web/mock runtime ever advances a row from `pending` back to `indexed`. This test
    // therefore proves the trigger fires and requeues the index (indexed/failed -> pending); the
    // pending -> indexed half of the design doc's transition happens only server-side, driven by
    // the async embedding worker, and is not observable through this mock. See the task-17 report
    // for the full explanation of why a stronger assertion is not possible here.
    await retrievalSection.getByRole("button", { name: "重建索引" }).click();
    const rebuildDialog = page.getByRole("dialog", { name: "重建检索索引" });
    await rebuildDialog.getByRole("button", { name: "确认重建" }).click();
    await expect(rebuildDialog).toBeHidden();

    await expect(statusValue(retrievalSection, "已索引")).toHaveText("0");
    await expect(statusValue(retrievalSection, "待索引")).toHaveText("17");
    await expect(statusValue(retrievalSection, "失败")).toHaveText("0");
  });
});
