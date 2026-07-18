import { renderToString } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import { RemoteWorkspaceSection, WorkspaceModeSelector } from "./create-session-workspace-sections";

describe("Create session remote workspace sections", () => {
  it("renders remote mode controls and known remote workspace history", () => {
    const modeHtml = renderToString(<WorkspaceModeSelector mode="remote" onModeChange={vi.fn()} />);
    const remoteHtml = renderToString(
      <RemoteWorkspaceSection
        knownRemoteWorkspaces={[{
          host: "remote.example.test",
          user: "dev",
          path: "/work/app",
          displayName: "Remote App",
          uri: "ssh://dev@remote.example.test/work/app",
          lastOpenedAt: "2026-07-18T00:00:00.000Z",
        }]}
        remoteDisplayName=""
        remoteHost=""
        remotePath=""
        remoteUser=""
        setRemoteDisplayName={vi.fn()}
        setRemoteHost={vi.fn()}
        setRemotePath={vi.fn()}
        setRemoteUser={vi.fn()}
      />,
    );

    expect(modeHtml).toContain("远端");
    expect(remoteHtml).toContain("主机");
    expect(remoteHtml).toContain("远端路径");
    expect(remoteHtml).toContain("Remote App");
    expect(remoteHtml).toContain("ssh://dev@remote.example.test/work/app");
  });
});
