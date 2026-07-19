import { describe, expect, it } from "vitest";
import { webSessionWorkspaceClient } from "./web-session-workspace-client";

describe("Web folder opener adapter", () => {
  it("keeps a deterministic catalog and rejects an omitted File Explorer fallback", async () => {
    const catalog = await webSessionWorkspaceClient.listFolderOpeners();
    expect(catalog.map((item) => item.id)).toEqual(["vscode", "file-explorer", "windows-terminal", "git-bash", "intellij-idea", "webstorm"]);
    await expect(webSessionWorkspaceClient.saveFolderOpenerPreferences({ configuredDefaultOpenerId: "git-bash", enabledOpenerIds: ["git-bash"] })).rejects.toThrow("File Explorer");
    const saved = await webSessionWorkspaceClient.saveFolderOpenerPreferences({ configuredDefaultOpenerId: "git-bash", enabledOpenerIds: ["git-bash", "file-explorer"] });
    expect(saved.enabledOpenerIds).toEqual(["git-bash", "file-explorer"]);
  });

  it("rejects an unavailable configured default", async () => {
    await expect(webSessionWorkspaceClient.saveFolderOpenerPreferences({ configuredDefaultOpenerId: "webstorm", enabledOpenerIds: ["file-explorer", "webstorm"] })).rejects.toThrow("available");
  });

  it("does not claim a native launch", async () => {
    await expect(webSessionWorkspaceClient.openSessionFolder("web-session", "vscode")).resolves.toEqual({ status: "unavailable", openerId: "vscode", reason: "web-runtime" });
  });
});
