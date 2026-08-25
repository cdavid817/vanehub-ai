// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import type { ReactElement } from "react";
import { I18nextProvider } from "react-i18next";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { AgentService } from "../services/agent-service";
import type {
  WorkspaceCapabilityState,
  WorkspaceInspectionCapabilities,
} from "../types/session-workspace";
import {
  useWorkspaceCapabilities,
  WorkspaceCapabilityNotice,
  WorkspaceWatchNotice,
} from "./workspace-capability-notice";

function mount(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { mutations: { retry: false }, queries: { retry: false } },
  });
  return render(
    <I18nextProvider i18n={i18n}>
      <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>
    </I18nextProvider>,
  );
}

function capabilities(
  overrides: Partial<WorkspaceInspectionCapabilities> = {},
): WorkspaceInspectionCapabilities {
  const available: WorkspaceCapabilityState = { available: true };
  return {
    provider: "local",
    listFiles: available,
    readTextFiles: available,
    searchFiles: available,
    gitStatus: available,
    gitDiff: available,
    watchMode: "event-derived",
    ...overrides,
  };
}

describe("workspace capability notices", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("shows nothing while a capability is available", () => {
    const { container } = mount(
      <WorkspaceCapabilityNotice capability={{ available: true }} />,
    );
    expect(container.textContent).toBe("");
  });

  it("explains a missing prerequisite and how to fix it", () => {
    mount(
      <WorkspaceCapabilityNotice
        capability={{
          available: false,
          reasonCode: "remote_ripgrep_missing",
          remediation: "remote_install_ripgrep",
        }}
        targetLabel="build-host"
      />,
    );

    expect(screen.getByText(/ripgrep is not installed/)).toBeTruthy();
    // "Search is unavailable" and "install ripgrep on the remote host" are different facts, and
    // only the second is something a reader can act on.
    expect(screen.getByText(/Install ripgrep on the remote host/)).toBeTruthy();
    // Which machine. Without it, the sentence reads as a fault in this application.
    expect(screen.getByText(/build-host/)).toBeTruthy();
  });

  /**
   * 11.13: a missing prerequisite must not take away the terminal.
   *
   * The Shell needs none of these — no helper, no Git, no ripgrep — so a reader looking at a panel
   * that cannot answer needs to be told the session is still reachable. The alternative reading is
   * that the whole host is gone.
   */
  it("says the Shell is still available whatever is missing", () => {
    for (const reason of [
      "remote_host_not_posix",
      "remote_git_missing",
      "remote_connection_unavailable",
    ]) {
      const { unmount } = mount(
        <WorkspaceCapabilityNotice capability={{ available: false, reasonCode: reason }} />,
      );
      expect(screen.getByText(/Shell for this session is still available/)).toBeTruthy();
      unmount();
    }
  });

  it("falls back to a general sentence for a code this build does not know", () => {
    mount(
      <WorkspaceCapabilityNotice
        capability={{ available: false, reasonCode: "invented_by_a_newer_backend" }}
      />,
    );

    // A backend can add a code before this build knows it, and showing the raw token would put an
    // untranslated identifier in front of a reader.
    expect(screen.getByText(/cannot be inspected right now/)).toBeTruthy();
    expect(screen.queryByText(/invented_by_a_newer_backend/)).toBeNull();
  });

  it("omits a local workspace's label because there is nothing to say", () => {
    mount(
      <WorkspaceCapabilityNotice capability={{ available: false, reasonCode: "remote_git_missing" }} />,
    );
    expect(screen.queryByText(/^On /)).toBeNull();
  });
});

describe("workspace watch notices", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("says nothing for a view that keeps itself up to date", () => {
    for (const watchMode of ["native", "event-derived"] as const) {
      const { container, unmount } = mount(
        <WorkspaceWatchNotice capabilities={capabilities({ watchMode })} />,
      );
      // The correct amount of interface for "this refreshes itself" is none.
      expect(container.textContent).toBe("");
      unmount();
    }
  });

  it("warns when a view can only be as fresh as its last poll", () => {
    mount(<WorkspaceWatchNotice capabilities={capabilities({ watchMode: "polling" })} />);
    expect(screen.getByText(/refreshed periodically/)).toBeTruthy();
  });

  it("warns when nothing will ever arrive on its own", () => {
    mount(<WorkspaceWatchNotice capabilities={capabilities({ watchMode: "none" })} />);
    // The reader has to know an external change is invisible until they press refresh.
    expect(screen.getByText(/will not appear until you refresh/)).toBeTruthy();
  });
});

describe("useWorkspaceCapabilities", () => {
  it("reads nothing without a session", async () => {
    const service = {
      getWorkspaceInspectionCapabilities: vi.fn(),
    } as unknown as AgentService;

    mount(<Probe sessionId={null} service={service} />);

    await waitFor(() =>
      expect(service.getWorkspaceInspectionCapabilities).not.toHaveBeenCalled(),
    );
  });

  it("reports the provider so a panel can say which machine answered", async () => {
    const service = {
      getWorkspaceInspectionCapabilities: vi.fn(async () =>
        capabilities({ provider: "ssh", targetLabel: "build-host" }),
      ),
    } as unknown as AgentService;

    mount(<Probe sessionId="session-1" service={service} />);

    // Waited on the content rather than the element: the probe renders before the query
    // settles, so finding it proves only that the component mounted.
    await waitFor(() =>
      expect(screen.getByTestId("probe").textContent).toBe("ssh:build-host"),
    );
  });
});

function Probe({ sessionId, service }: { sessionId: string | null; service: AgentService }) {
  const { capabilities: answer } = useWorkspaceCapabilities(sessionId, service);
  return (
    <output data-testid="probe">{`${answer?.provider ?? "-"}:${answer?.targetLabel ?? "-"}`}</output>
  );
}
