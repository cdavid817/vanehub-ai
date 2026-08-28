import { describe, expect, it } from "vitest";
import { webSessionWorkspaceClient } from "./web-session-workspace-client";

describe("Web session workspace adapter", () => {
  it("returns deterministic file, document, Git, and redacted log fixtures", async () => {
    const root = await webSessionWorkspaceClient.listSessionDirectory("session-1", "");
    expect(root.items.map((entry) => entry.name)).toContain("README.md");
    expect((await webSessionWorkspaceClient.readSessionFile("session-1", "README.md")).content).toContain("Web Preview");
    expect((await webSessionWorkspaceClient.listSessionDocuments("session-1")).items).toHaveLength(3);
    expect((await webSessionWorkspaceClient.getSessionGitStatus("session-1")).isGit).toBe(true);
    expect((await webSessionWorkspaceClient.getSessionGitDiff("session-1", "src/main.ts", "staged")).source).toBe("staged");
    const logs = await webSessionWorkspaceClient.listSessionLogs({ sessionId: "session-1", levels: ["warn"], search: "retry" });
    expect(logs.items).toHaveLength(1);
    expect(JSON.stringify(logs.items)).not.toContain("secret");
    expect(await webSessionWorkspaceClient.exportSessionLogs({ sessionId: "session-1", levels: [], search: "" })).toEqual({ status: "unavailable", path: null });
  });

  it("serves mention candidates covering source files with the native ordering contract", async () => {
    const listing = await webSessionWorkspaceClient.searchSessionFiles("session-1", "main", 8);
    // The Documents tab fixture holds only Markdown and text; mention search must reach further.
    expect(listing.items.map((entry) => entry.path)).toContain("src/main.ts");
    const ranked = await webSessionWorkspaceClient.searchSessionFiles("session-1", "notes", 8);
    expect(ranked.items[0].path).toBe("docs/notes.txt");
    const capped = await webSessionWorkspaceClient.searchSessionFiles("session-1", "", 2);
    expect(capped.items).toHaveLength(2);
    expect(capped.truncated).toBe(true);
  });

});
// Shell I/O left this adapter with the one-view service it belonged to. The retained Session Shell
// mock and its lifecycle are covered by `session-shell-client.test.ts`, against the interface the
// Shell tab actually uses.

