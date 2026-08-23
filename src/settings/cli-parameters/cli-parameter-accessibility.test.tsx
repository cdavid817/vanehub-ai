// @vitest-environment jsdom
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../i18n";
import { CliParameterListControl } from "./cli-parameter-list-control";
import { CliParameterToolbar } from "./cli-parameter-toolbar";
import { cliParameterDefinitions } from "../../services/cli-parameter-registry";
import type { CliParameterSelection } from "../../types/cli-parameter";

const selectProjectDirectory = vi.fn();

vi.mock("../../services/runtime-agent-client", () => ({
  agentService: { selectProjectDirectory: () => selectProjectDirectory() },
}));

const includeDirectories = cliParameterDefinitions("gemini-cli").find(
  (definition) => definition.id === "includeDirectories",
)!;

function ListHarness({ entries }: { entries: string[] }) {
  return (
    <CliParameterListControl
      definition={includeDirectories}
      disabled={false}
      entries={entries}
      onChange={(selection) => {
        (globalThis as unknown as { __lastChange?: CliParameterSelection }).__lastChange = selection;
      }}
    />
  );
}

function lastChange(): CliParameterSelection | undefined {
  return (globalThis as unknown as { __lastChange?: CliParameterSelection }).__lastChange;
}

describe("CLI parameter accessibility", () => {
  it("reorders and removes list entries with buttons rather than a drag handle", async () => {
    const user = userEvent.setup();
    render(<ListHarness entries={["C:/first", "C:/second"]} />);

    await user.click(screen.getByRole("button", { name: "将 C:/second 上移" }));
    expect(lastChange()).toEqual({ state: "value", value: ["C:/second", "C:/first"] });

    // The harness is uncontrolled, so each action still starts from the same prop; removing the
    // first entry therefore leaves the second.
    await user.click(screen.getByRole("button", { name: "移除 C:/first" }));
    expect(lastChange()).toEqual({ state: "value", value: ["C:/second"] });
  });

  it("adds a directory through the service boundary rather than a Tauri call", async () => {
    const user = userEvent.setup();
    selectProjectDirectory.mockResolvedValueOnce("D:/work space");
    render(<ListHarness entries={["C:/first"]} />);

    await user.click(screen.getByRole("button", { name: "选择目录" }));

    await waitFor(() =>
      expect(lastChange()).toEqual({ state: "value", value: ["C:/first", "D:/work space"] }),
    );
  });

  it("clears the list to inherit rather than to an empty value", async () => {
    const user = userEvent.setup();
    render(<ListHarness entries={["C:/only"]} />);

    await user.click(screen.getByRole("button", { name: "移除 C:/only" }));

    expect(lastChange()).toEqual({ state: "inherit" });
  });

  it("keeps focus on a filter control after filtering", async () => {
    const user = userEvent.setup();
    function Harness() {
      return (
        <CliParameterToolbar
          filter="all"
          onFilterChange={() => undefined}
          onQueryChange={() => undefined}
          onScopeChange={() => undefined}
          query=""
          scope="chat"
        />
      );
    }
    render(<Harness />);

    const filters = screen.getByRole("group", { name: "筛选" });
    const modified = within(filters).getByRole("button", { name: "已修改" });
    await user.click(modified);

    // The toolbar is outside the field list, so filtering never remounts the control the user is
    // operating and focus survives.
    expect(document.activeElement).toBe(modified);
  });

  it("names the scope group and marks the active scope for assistive technology", () => {
    render(
      <CliParameterToolbar
        filter="all"
        onFilterChange={() => undefined}
        onQueryChange={() => undefined}
        onScopeChange={() => undefined}
        query=""
        scope="interactive"
      />,
    );

    const scopes = screen.getByRole("group", { name: "启动范围" });
    expect(
      within(scopes).getByRole("button", { name: "交互式" }).getAttribute("aria-pressed"),
    ).toBe("true");
    expect(within(scopes).getByRole("button", { name: "对话" }).getAttribute("aria-pressed")).toBe(
      "false",
    );
  });
});
