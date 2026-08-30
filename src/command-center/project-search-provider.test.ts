import { afterEach, describe, expect, it, vi } from "vitest";
import { agentService } from "../services/runtime-agent-client";
import { sshConnectionService } from "../services/runtime-ssh-connection-client";
import type { KnownProject } from "../types/agent";
import type { SshConnection } from "../types/ssh-connection";
import { projectSearchProvider } from "./project-search-provider";
import type { WorkbenchSearchRequest } from "./command-center-types";

afterEach(() => vi.restoreAllMocks());

function project(overrides: Partial<KnownProject> = {}): KnownProject {
  return { path: "D:/code/example", displayName: "example", isGit: true, lastOpenedAt: "2026-08-01T00:00:00.000Z", ...overrides };
}

function connection(overrides: Partial<SshConnection> = {}): SshConnection {
  return {
    id: "conn-1", name: "build box", host: "build.example.test", port: 22, user: "deploy",
    defaultPath: "/srv/app", authMode: "key", keyPath: "/home/deploy/.ssh/id_ed25519", hasPassword: false,
    revision: 1, hostTrust: null, testStatus: "succeeded", lastConnectedAt: "2026-08-01T00:00:00.000Z",
    lastError: null, createdAt: "2026-01-01T00:00:00.000Z", updatedAt: "2026-08-01T00:00:00.000Z",
    ...overrides,
  };
}

function request(overrides: Partial<WorkbenchSearchRequest> = {}): WorkbenchSearchRequest {
  return { query: "", scopes: ["project"], limit: 20, signal: new AbortController().signal, ...overrides };
}

describe("projectSearchProvider", () => {
  it("supports only the project scope", () => {
    expect(projectSearchProvider.supports("project")).toBe(true);
    expect(projectSearchProvider.supports("session")).toBe(false);
    expect(projectSearchProvider.supports("run")).toBe(false);
  });

  it("maps a local project to a search result", async () => {
    vi.spyOn(agentService, "listKnownProjects").mockResolvedValue([project()]);
    vi.spyOn(sshConnectionService, "listConnections").mockResolvedValue([]);

    const page = await projectSearchProvider.search(request({ query: "example" }));

    expect(page).toEqual({
      items: [{
        key: "D:/code/example", kind: "project", title: "example", subtitle: "D:/code/example",
        route: { destination: "projects", projectId: "D:/code/example" }, updatedAt: "2026-08-01T00:00:00.000Z",
      }],
      nextCursor: null,
    });
  });

  it("maps an SSH connection to a search result", async () => {
    vi.spyOn(agentService, "listKnownProjects").mockResolvedValue([]);
    vi.spyOn(sshConnectionService, "listConnections").mockResolvedValue([connection()]);

    const page = await projectSearchProvider.search(request({ query: "build" }));

    expect(page.items).toEqual([{
      key: "conn-1", kind: "project", title: "build box", subtitle: "deploy@build.example.test",
      route: { destination: "projects", projectId: "conn-1" },
    }]);
  });

  it("filters by query, case-insensitively, across both sources", async () => {
    vi.spyOn(agentService, "listKnownProjects").mockResolvedValue([project({ displayName: "Alpha" }), project({ path: "D:/code/beta", displayName: "Beta" })]);
    vi.spyOn(sshConnectionService, "listConnections").mockResolvedValue([connection({ id: "conn-alpha", name: "Alpha box" }), connection({ id: "conn-gamma", name: "Gamma box" })]);

    const page = await projectSearchProvider.search(request({ query: "alpha" }));

    expect(page.items.map((item) => item.key).sort()).toEqual(["D:/code/example", "conn-alpha"].sort());
  });

  it("matches a project by its path, not just its display name", async () => {
    vi.spyOn(agentService, "listKnownProjects").mockResolvedValue([project({ path: "D:/code/needle-project", displayName: "unrelated" })]);
    vi.spyOn(sshConnectionService, "listConnections").mockResolvedValue([]);

    const page = await projectSearchProvider.search(request({ query: "needle" }));

    expect(page.items).toHaveLength(1);
  });

  it("returns nothing for an empty query", async () => {
    vi.spyOn(agentService, "listKnownProjects").mockResolvedValue([project()]);
    vi.spyOn(sshConnectionService, "listConnections").mockResolvedValue([connection()]);

    const page = await projectSearchProvider.search(request({ query: "" }));

    expect(page.items).toEqual([]);
  });

  it("returns an empty page when nothing matches", async () => {
    vi.spyOn(agentService, "listKnownProjects").mockResolvedValue([project()]);
    vi.spyOn(sshConnectionService, "listConnections").mockResolvedValue([connection()]);

    const page = await projectSearchProvider.search(request({ query: "no-such-thing-anywhere" }));

    expect(page).toEqual({ items: [], nextCursor: null });
  });

  it("respects limit on the combined total across both sources", async () => {
    vi.spyOn(agentService, "listKnownProjects").mockResolvedValue([
      project({ path: "D:/code/match-1", displayName: "match one" }),
      project({ path: "D:/code/match-2", displayName: "match two" }),
    ]);
    vi.spyOn(sshConnectionService, "listConnections").mockResolvedValue([
      connection({ id: "conn-match-1", name: "match three" }),
      connection({ id: "conn-match-2", name: "match four" }),
    ]);

    const page = await projectSearchProvider.search(request({ query: "match", limit: 2 }));

    expect(page.items).toHaveLength(2);
  });

  it("never leaks an SSH connection's lastError text", async () => {
    vi.spyOn(agentService, "listKnownProjects").mockResolvedValue([]);
    vi.spyOn(sshConnectionService, "listConnections").mockResolvedValue([
      connection({ name: "flaky box", lastError: "SECRET_ERROR_TEXT: auth failed for deploy@build.example.test" }),
    ]);

    const page = await projectSearchProvider.search(request({ query: "flaky" }));

    expect(JSON.stringify(page)).not.toContain("SECRET_ERROR_TEXT");
  });
});
