// @vitest-environment jsdom

import { readFileSync } from "node:fs";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { renderToString } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import en from "../../i18n/locales/en.json";
import type {
  PluginIntegrationDefinition,
  PluginIntegrationOverview,
  PluginIntegrationState,
} from "../../types/plugin-integration";
import { filterPluginIntegrations, PluginIntegrationsPage } from "./plugin-integrations-page";

const definitions: PluginIntegrationDefinition[] = [
  {
    id: "github",
    nameKey: "plugins.github.name",
    descriptionKey: "plugins.github.description",
    version: "1.0.0",
    provider: "GitHub",
    icon: "github",
    docsUrl: "https://cli.github.com/manual/gh_auth_login",
    setupSteps: [{ id: "auth", labelKey: "plugins.github.setup.auth" }],
  },
];

const states: PluginIntegrationState[] = [
  {
    integrationId: "github",
    status: "unavailable",
    configured: false,
    canTest: false,
    lastCheckedAt: null,
    statusReasonKey: "plugins.environment.desktopOnly",
    message: null,
  },
];

const translate = (key: string) => en[key as keyof typeof en] ?? key;

describe("PluginIntegrationsPage", () => {
  it("filters by localized GitHub setup and status text", () => {
    expect(filterPluginIntegrations(definitions, states, "GitHub", translate)).toHaveLength(1);
    expect(filterPluginIntegrations(definitions, states, "authentication", translate)).toHaveLength(1);
    expect(filterPluginIntegrations(definitions, states, "paddleocr", translate)).toHaveLength(0);
  });

  it("renders Web mock desktop-only limitation from the service boundary", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["plugin-integrations", "overview"], {
      definitions,
      states,
      environment: {
        runtime: "web-mock" as const,
        nativeChecksAvailable: false,
        reasonKey: "plugins.environment.desktopOnly",
      },
    });
    const service = {
      async getOverview() {
        return {
          definitions,
          states,
          environment: {
            runtime: "web-mock" as const,
            nativeChecksAvailable: false,
            reasonKey: "plugins.environment.desktopOnly",
          },
        };
      },
      async refresh() {
        return this.getOverview();
      },
      async testReadiness() {
        return {
          integrationId: "github" as const,
          status: "unavailable" as const,
          configured: false,
          message: "plugins.environment.desktopOnly",
          checkedAt: "preview",
        };
      },
    };

    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <PluginIntegrationsPage searchTerm="" service={service} />
      </QueryClientProvider>,
    );

    expect(html).toContain("plugins.environment.desktopOnly");
    expect(html).toContain("plugin-card-github");
  });

  it("uses semantic styles without theme-name branches or direct Tauri calls", () => {
    const pageSource = readFileSync("src/settings/pages/plugin-integrations-page.tsx", "utf8");
    const cardSource = readFileSync("src/settings/pages/plugins/plugin-integration-card.tsx", "utf8");
    for (const source of [pageSource, cardSource]) {
      expect(source).not.toContain("@tauri-apps/api");
      expect(source).not.toContain("invoke(");
      expect(source).not.toMatch(/theme\s*===/);
    }
    // Task 12.18: the per-item card (mirrors ssh/ssh-connection-card.tsx), not this page file
    // itself, now owns the ucd-panel card surface -- migrated out of this file along with the
    // rest of the per-item markup.
    expect(cardSource).toContain("ucd-panel");
  });

  it("shows the page icon and exactly one primary header action, with no More menu needed", async () => {
    const queryClient = new QueryClient();
    const service = {
      async getOverview() {
        return { definitions, states, environment: { runtime: "tauri" as const, nativeChecksAvailable: true, reasonKey: null } };
      },
      async refresh() {
        return this.getOverview();
      },
      async testReadiness(): Promise<never> {
        throw new Error("unused");
      },
    };

    render(
      <QueryClientProvider client={queryClient}>
        <PluginIntegrationsPage searchTerm="" service={service} />
      </QueryClientProvider>,
    );
    await screen.findByTestId("plugin-card-github");

    const header = document.querySelector(".border-b.border-border-subtle");
    expect(header).toBeTruthy();
    expect(header?.querySelector("svg")).toBeTruthy();
    // Task 12.18: Plugin Integrations only ever had one always-visible header action (Refresh),
    // so it becomes the shared PageHeader's single primaryAction directly -- no moreMenuItems
    // (and therefore no More trigger) is needed at all, unlike ssh-connections-page.tsx's Add+Refresh.
    expect(within(header as HTMLElement).getByRole("button", { name: "plugins.refresh" })).toBeTruthy();
    expect(within(header as HTMLElement).queryAllByRole("button")).toHaveLength(1);
  });

  it("collapses per-card Docs/Test actions behind one ActionMenu instead of two buttons", async () => {
    const queryClient = new QueryClient();
    const service = {
      async getOverview() {
        return { definitions, states, environment: { runtime: "tauri" as const, nativeChecksAvailable: true, reasonKey: null } };
      },
      async refresh() {
        return this.getOverview();
      },
      async testReadiness(): Promise<never> {
        throw new Error("unused");
      },
    };

    render(
      <QueryClientProvider client={queryClient}>
        <PluginIntegrationsPage searchTerm="" service={service} />
      </QueryClientProvider>,
    );
    const card = await screen.findByTestId("plugin-card-github");

    expect(within(card).queryByRole("button", { name: "plugins.action.docs" })).toBeNull();
    expect(within(card).queryByRole("button", { name: "plugins.action.test" })).toBeNull();
    // Task 12.19: the ActionMenu trigger plus one deliberately-standalone Copy Diagnostics button
    // (matching IM/Local Media's own precedent of never hiding this action behind a menu) -- still
    // exactly zero separate Docs/Test buttons, which is what this test actually guards.
    expect(within(card).getAllByRole("button")).toHaveLength(2);

    fireEvent.click(within(card).getByRole("button", { name: "plugins.rowActions" }));
    expect(within(card).getByRole("menuitem", { name: "plugins.action.docs" })).toBeTruthy();
    expect(within(card).getByRole("menuitem", { name: "plugins.action.test" })).toBeTruthy();
  });

  it("renders the shared AsyncBoundary error state when the overview query fails, with a working retry", async () => {
    // react-query's default `retry: 3` would otherwise keep this query retrying with backoff for
    // several seconds before ever reaching `isError`, well past this test's own `waitFor` budget.
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const getOverview = vi.fn(async (): Promise<PluginIntegrationOverview> => {
      throw new Error("network down");
    });
    const service = {
      getOverview,
      async refresh() {
        return this.getOverview();
      },
      async testReadiness(): Promise<never> {
        throw new Error("unused");
      },
    };

    render(
      <QueryClientProvider client={queryClient}>
        <PluginIntegrationsPage searchTerm="" service={service} />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("network down"));
    getOverview.mockImplementationOnce(async () => ({
      definitions,
      states,
      environment: { runtime: "tauri" as const, nativeChecksAvailable: true, reasonKey: null },
    }));
    fireEvent.click(screen.getByRole("button", { name: "featureLoad.retry" }));
    await screen.findByTestId("plugin-card-github");
  });

  it("reports an error status for its nav entry once a readiness test fails, and null while healthy (task 12.16)", async () => {
    const queryClient = new QueryClient();
    const healthyStates: PluginIntegrationState[] = [
      {
        integrationId: "github",
        status: "not-configured",
        configured: false,
        canTest: true,
        lastCheckedAt: null,
        statusReasonKey: "plugins.statusReason.notChecked",
        message: null,
      },
    ];
    const onStatusChange = vi.fn();
    const service = {
      async getOverview() {
        return {
          definitions,
          states: healthyStates,
          environment: {
            runtime: "tauri" as const,
            nativeChecksAvailable: true,
            reasonKey: null,
          },
        };
      },
      async refresh() {
        return this.getOverview();
      },
      async testReadiness() {
        throw new Error("boom");
      },
    };

    render(
      <QueryClientProvider client={queryClient}>
        <PluginIntegrationsPage onStatusChange={onStatusChange} searchTerm="" service={service} />
      </QueryClientProvider>,
    );

    const card = await screen.findByTestId("plugin-card-github");
    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith(null));

    // Task 12.18: Test now lives behind the card's single ActionMenu rather than as its own
    // directly-clickable button -- open the menu, then activate the Test item inside it. Scoped by
    // name since task 12.19 added a second, standalone Copy Diagnostics button to this same card.
    fireEvent.click(within(card).getByRole("button", { name: "plugins.rowActions" }));
    fireEvent.click(within(card).getByRole("menuitem", { name: "plugins.action.test" }));

    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith({
      kind: "error",
      labelKey: "plugins.pageStatus.error",
    }));
  });
});
