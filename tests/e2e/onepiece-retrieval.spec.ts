import { expect, test, type Locator, type Page } from "@playwright/test";

async function openAgentConfigurations(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /^(Agent 配置|Agent Configurations)$/ }).click();
  await page.getByRole("tab", { name: /OnePiece/ }).click();
}

async function openOnePieceParameters(page: Page) {
  await page.getByRole("button", { name: /^(CLI 参数|CLI Parameters)$/ }).click();
  await page.getByRole("button", { name: "OnePiece" }).click();
  await expect(page.getByRole("region", { name: /检索索引配置|Retrieval index configuration/ })).toBeVisible();
}

async function createEmbeddingProvider(page: Page, name: string) {
  await openAgentConfigurations(page);
  const panel = page.getByRole("tabpanel", { name: "OnePiece" });
  await panel.getByRole("button", { name: "新增配置" }).first().click();
  const dialog = page.getByRole("dialog", { name: "新增 OnePiece 配置" });
  await dialog.getByRole("button", { name: /OpenRouter/ }).click();
  await dialog.getByLabel("配置名称").fill(name);
  await dialog.getByLabel("模型", { exact: true }).selectOption("anthropic/claude-sonnet-4.6");
  await dialog.getByLabel("API 密钥").fill("not-persisted-playwright-secret");
  await dialog.getByRole("button", { name: "保存 OnePiece" }).click();
  await expect(panel.getByRole("heading", { name })).toBeVisible();
}

function agentButton(dialog: Locator, name: string) {
  return dialog.locator("button").filter({ hasText: name }).first();
}

test.describe("OnePiece retrieval configuration", () => {
  test("manages retrieval and Embedding parameters from CLI Parameter Management", async ({ page }) => {
    await createEmbeddingProvider(page, "检索 Embedding 源");
    const providerPanel = page.getByRole("tabpanel", { name: "OnePiece" });
    await expect(providerPanel.getByRole("region", { name: "检索索引配置" })).toHaveCount(0);

    await openOnePieceParameters(page);
    const retrieval = page.getByRole("region", { name: "检索索引配置" });
    await retrieval.getByRole("combobox", { name: "自动项目代码索引" }).selectOption("semantic");
    await retrieval.getByRole("combobox", { name: "Embedding 来源" }).selectOption({ label: "检索 Embedding 源" });
    await retrieval.getByRole("combobox", { name: "Embedding 模型" }).selectOption("text-embedding-3-small");
    await retrieval.getByRole("button", { name: "保存检索配置" }).click();
    await expect(retrieval.getByRole("status")).toHaveText("检索配置已保存。");

    await expect(retrieval.getByText("索引状态", { exact: true })).toHaveCount(0);
    await expect(retrieval.getByRole("button", { name: "重建索引" })).toHaveCount(0);
  });

  test("follows a local OnePiece session in the information panel without Embedding", async ({ page }) => {
    await page.goto("/");
    await page.evaluate(async () => {
      const module = await import("/src/services/web-agent-client.ts");
      await module.webAgentClient.saveOnePieceProviderConfig({
        provider: "Anthropic",
        modelId: "claude-opus-4-8",
        interfaceFormat: "anthropic",
        baseUrl: null,
        apiKey: "playwright-local-only-key",
      });
    });
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await openOnePieceParameters(page);
    const retrieval = page.getByRole("region", { name: "检索索引配置" });
    await retrieval.getByRole("combobox", { name: "自动项目代码索引" }).selectOption("local");
    await expect.poll(() => page.evaluate(async () => {
      const module = await import("/src/services/web-agent-client.ts");
      return (await module.webAgentClient.getRetrievalConfiguration()).automaticCodeIndexMode;
    })).toBe("local");
    await expect(retrieval.getByRole("combobox", { name: "Embedding 来源" })).toHaveCount(0);

    await page.getByRole("button", { name: "返回", exact: true }).click();
    await page.getByRole("button", { name: /新建/ }).click();
    const createDialog = page.getByRole("dialog");
    await agentButton(createDialog, "OnePiece").click();
    await createDialog.getByPlaceholder(/code.*project/).fill("D:\\code\\automatic-local");
    await createDialog.getByPlaceholder("新会话").fill("本地代码索引会话");
    await createDialog.getByRole("button", { name: "创建", exact: true }).click();

    const infoPanel = page.locator("aside").filter({ hasText: /信息面板|Info Panel/ });
    await infoPanel.getByRole("tab", { name: "代码索引", exact: true }).click();
    await expect(infoPanel.getByText("D:\\code\\automatic-local", { exact: true }).last()).toBeVisible();
    await expect(infoPanel.getByText("仅本地", { exact: true })).toBeVisible();
    await expect(infoPanel.getByText("扫描中", { exact: true })).toBeVisible();
    await infoPanel.getByRole("button", { name: "刷新文件清单" }).click();
    await expect(infoPanel.getByText("解析中", { exact: true })).toBeVisible();
    await infoPanel.getByRole("button", { name: "刷新文件清单" }).click();
    await expect(infoPanel.getByText("已就绪", { exact: true }).first()).toBeVisible();
    await expect(infoPanel.getByText("预计请求").locator("xpath=following-sibling::dd[1]")).toHaveText("0");
    await expect(infoPanel.getByRole("button", { name: "查看并确认" })).toHaveCount(0);

    const scopedSearch = await page.evaluate(async () => {
      const module = await import("/src/services/web-agent-client.ts");
      const workspace = (await module.webAgentClient.listCodeIndexWorkspaces())[0];
      return module.searchWebCodeIndex(workspace.workspaceId, "handle_login");
    });
    expect(scopedSearch).toMatchObject([{ filePath: "src/auth.ts", symbolName: "handle_login", matchedVia: "keyword" }]);
  });
});
