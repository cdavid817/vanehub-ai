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

  it("simulates shell I/O and supports cleanup without a native process", async () => {
    const shell = await webSessionWorkspaceClient.createShell({ sessionId: "session-1", rows: 24, cols: 80 });
    expect(shell.capability).toBe("simulated");
    const events: string[] = [];
    const unsubscribe = await webSessionWorkspaceClient.subscribeShellEvents(shell.shellId, (event) => {
      if (event.type === "output") events.push(event.content);
    });
    await webSessionWorkspaceClient.writeShellInput(shell.shellId, "pwd\r");
    await webSessionWorkspaceClient.resetShellDirectory(shell.shellId);
    expect(events.join("\n")).toContain("WEB MOCK");
    unsubscribe();
    await webSessionWorkspaceClient.killShell(shell.shellId);
    await expect(webSessionWorkspaceClient.resizeShell({ shellId: shell.shellId, rows: 30, cols: 100 })).rejects.toThrow("not found");
  });
});

