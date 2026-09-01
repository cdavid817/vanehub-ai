// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderToString } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import "../../i18n";
import { managedCliAgentIds, type ManagedCliAgentId } from "../../types/agent";
import {
  cliParameterCatalogVersion,
  defaultCliParameterSelections,
  editableCliParameterDefinitions,
} from "../../services/cli-parameter-registry";
import { renderCliParameterSegments } from "../../services/cli-parameter-renderer";
import type {
  CliParameterProfile,
  PreviewCliParameterProfileInput,
} from "../../types/cli-parameter-profile";
import { CliParametersPage } from "./cli-parameters-page";

// The page must never reach a real adapter from a component test. Preview echoes its input so the
// panel has something to render; nothing here writes.
vi.mock("../../services/runtime-agent-client", () => ({
  agentService: {
    listCliParameterProfiles: vi.fn(() => Promise.resolve([])),
    previewCliParameterProfile: vi.fn((input: PreviewCliParameterProfileInput) =>
      Promise.resolve({
        agentId: input.agentId,
        catalogVersion: input.catalogVersion,
        scope: input.scope,
        normalizedSelections: input.selections,
        segments: { global: [], invocation: [] },
        diagnostics: [],
      }),
    ),
    saveCliParameterProfile: vi.fn(),
    resetCliParameterProfile: vi.fn(),
  },
}));

function profile(agentId: ManagedCliAgentId): CliParameterProfile {
  const definitions = editableCliParameterDefinitions(agentId);
  const selections = defaultCliParameterSelections(agentId);
  return {
    agentId,
    catalogVersion: cliParameterCatalogVersion,
    revision: 0,
    updatedAt: null,
    installation: { installed: true, runnable: true, conflict: false, version: "2.1.237" },
    fields: definitions.map((definition) => ({
      definition,
      support: { state: "supported" },
      optionSupport: {},
    })),
    selections,
    savedPreviews: {
      chat: renderCliParameterSegments(definitions, selections, "chat"),
      interactive: renderCliParameterSegments(definitions, selections, "interactive"),
    },
    diagnostics: [],
  };
}

function seededClient() {
  // The seeded profiles are the fixture. Without pinning staleness react-query refetches on mount
  // and the mocked adapter's empty list replaces them, which silently empties the page.
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, staleTime: Infinity, refetchOnMount: false } },
  });
  client.setQueryData(
    ["cli-parameter-profiles"],
    [...managedCliAgentIds].reverse().map(profile),
  );
  return client;
}

function markup(searchTerm = "") {
  return renderToString(
    <QueryClientProvider client={seededClient()}>
      <CliParametersPage searchTerm={searchTerm} />
    </QueryClientProvider>,
  );
}

function mount() {
  return render(
    <QueryClientProvider client={seededClient()}>
      <CliParametersPage searchTerm="" />
    </QueryClientProvider>,
  );
}

describe("CliParametersPage", () => {
  it("shows an external-CLI rail in the shared settings order with lifecycle state", () => {
    const html = markup();

    const positions = ["Claude Code", "Codex CLI", "OpenCode", "Antigravity CLI", "Gemini CLI"].map(
      (label) => html.indexOf(`>${label}<`),
    );
    expect(positions.every((position) => position >= 0)).toBe(true);
    expect(positions).toEqual([...positions].sort((left, right) => left - right));
    expect(html).toContain("版本 2.1.237");
    // OnePiece is linked, not tabbed: this page does not own its parameters.
    expect(html).toContain("Agent 配置");
  });

  it("renders registry categories and never a policy-governed parameter", () => {
    const html = markup();

    expect(html).toContain("模型与推理");
    expect(html).toContain("--model");
    expect(html).not.toContain("--permission-mode");
    expect(html).not.toContain("--dangerously-skip-permissions");
  });

  it("previews tokens rather than a joined shell command", () => {
    const html = markup();

    expect(html).toContain("安全参数预览");
    expect(html).toContain("复制 argv JSON");
    // A joined preview would put the executable and a space-separated string on screen.
    expect(html).not.toContain("claude --model");
  });

  it("keeps a custom text box from submitting an empty value", async () => {
    const user = userEvent.setup();
    mount();

    const modelSelect = await screen.findByLabelText("模型", { exact: true });
    await user.selectOptions(modelSelect, "__custom__");

    // Choosing Custom changed the editor only, so nothing is dirty yet -- the draft bar (dirtyCount
    // gated) has nothing to show until a value actually differs from the baseline.
    expect(screen.queryByRole("button", { name: "保存" })).toBeNull();

    const custom = screen.getByLabelText("输入您的 CLI 支持的模型标识符");
    await user.type(custom, "claude-opus-5");
    const save = await screen.findByRole("button", { name: "保存" });
    expect((save as HTMLButtonElement).disabled).toBe(false);

    await user.clear(custom);
    // An empty box is invalid, not an empty value: the last typed value stays dirty against the
    // baseline (so the bar stays visible) but save is refused again.
    await waitFor(() => expect((save as HTMLButtonElement).disabled).toBe(true));
  });

  it("does not carry transient custom text from one CLI to another", async () => {
    const user = userEvent.setup();
    mount();

    const modelSelect = await screen.findByLabelText("模型", { exact: true });
    await user.selectOptions(modelSelect, "__custom__");
    await user.type(screen.getByLabelText("输入您的 CLI 支持的模型标识符"), "claude-opus-5");

    const rail = screen.getByRole("navigation", { name: "受管 CLI" });
    await user.click(within(rail).getByRole("button", { name: /Codex CLI/ }));

    const codexModel = await screen.findByLabelText("模型", { exact: true });
    expect((codexModel as HTMLSelectElement).value).toBe("__inherit__");
    expect(screen.queryByDisplayValue("claude-opus-5")).toBeNull();
  });

  it("shows a different field set for each launch scope", async () => {
    const user = userEvent.setup();
    mount();

    // `--bare` is chat-only and `--chrome` is interactive-only, so the two scopes cannot both be
    // showing the same list.
    await screen.findByLabelText("\u6a21\u578b", { exact: true });
    expect(screen.queryByText("--bare")).not.toBeNull();
    expect(screen.queryByText("--chrome")).toBeNull();

    await user.click(screen.getByRole("button", { name: "\u4ea4\u4e92\u5f0f" }));

    await waitFor(() => expect(screen.queryByText("--chrome")).not.toBeNull());
    expect(screen.queryByText("--bare")).toBeNull();
  });

  it("keeps the draft and reports the code when a save is rejected", async () => {
    const user = userEvent.setup();
    const { agentService } = await import("../../services/runtime-agent-client");
    vi.mocked(agentService.saveCliParameterProfile).mockRejectedValueOnce({
      code: "CLI_PARAMETER_REVISION_CONFLICT",
      agentId: "claude-code",
    });
    mount();

    await user.selectOptions(
      await screen.findByLabelText("\u6a21\u578b", { exact: true }),
      "opus",
    );
    await user.click(screen.getByRole("button", { name: "\u4fdd\u5b58" }));

    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeNull());
    // The draft is not thrown away by a rejected write.
    expect(
      (screen.getByLabelText("\u6a21\u578b", { exact: true }) as HTMLSelectElement).value,
    ).toBe("opus");
  });

  it("keeps each CLI's draft while switching between them", async () => {
    const user = userEvent.setup();
    mount();

    const modelSelect = await screen.findByLabelText("模型", { exact: true });
    await user.selectOptions(modelSelect, "opus");

    const rail = screen.getByRole("navigation", { name: "受管 CLI" });
    await user.click(within(rail).getByRole("button", { name: /Codex CLI/ }));
    await user.click(within(rail).getByRole("button", { name: /Claude Code/ }));

    await waitFor(() =>
      expect(
        (screen.getByLabelText("模型", { exact: true }) as HTMLSelectElement).value,
      ).toBe("opus"),
    );
  });
});
