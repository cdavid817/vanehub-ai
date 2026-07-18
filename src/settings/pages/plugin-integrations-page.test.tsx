import { readFileSync } from "node:fs";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import en from "../../i18n/locales/en.json";
import type { PluginIntegrationDefinition, PluginIntegrationState } from "../../types/plugin-integration";
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
    const source = readFileSync("src/settings/pages/plugin-integrations-page.tsx", "utf8");
    expect(source).not.toContain("@tauri-apps/api");
    expect(source).not.toContain("invoke(");
    expect(source).not.toMatch(/theme\s*===/);
    expect(source).toContain("ucd-panel");
  });
});
