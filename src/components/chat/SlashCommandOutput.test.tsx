// @vitest-environment jsdom

import { beforeAll, describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import { SlashCommandOutput } from "./SlashCommandOutput";

describe("SlashCommandOutput", () => {
  // The app defaults to zh-CN; pin English so the assertions below can check real copy
  // instead of translation keys (see ModeSelect.test.tsx for the same pattern).
  beforeAll(async () => activateAppLanguage("en"));

  it("renders nothing when there is no output", () => {
    const { container } = renderWithAppProviders(<SlashCommandOutput output={null} onDismiss={() => undefined} />);
    expect(container.firstChild).toBeNull();
  });

  it("translates the title and each message", () => {
    renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={() => undefined}
        output={{ titleKey: "slash.output.applied", tone: "info", messages: [{ key: "slash.output.mode", params: { value: "plan" } }] }}
      />,
    );
    expect(screen.getByTestId("slash-command-output")).toBeTruthy();
    expect(screen.getByText("Applied")).toBeTruthy();
    expect(screen.getByText("Execution mode: plan")).toBeTruthy();
    // The info tone must not pick up the error tone's visual treatment (see the tone test below).
    expect(screen.getByTestId("slash-command-output").className).not.toContain("destructive");
  });

  it("translates a help entry's descriptionKey parameter before interpolating", () => {
    renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={() => undefined}
        output={{
          titleKey: "slash.output.helpTitle", tone: "info",
          messages: [{ key: "slash.output.helpEntry", params: { invocation: "/status", descriptionKey: "slash.command.status.description" } }],
        }}
      />,
    );
    expect(screen.getByText("/status — Show the current runtime switches")).toBeTruthy();
  });

  it("renders a literal `description` param unmangled instead of resolving it as a key", () => {
    // Regression test for the two-pass resolution keying off the param NAME (`descriptionKey`)
    // rather than a coincidental value shape. Before that rename, any param named `description`
    // was run through `t()` — this uses a value that is itself a real key ("slash.output.dismiss")
    // so a wrongly-applied second pass would swap in that key's translation ("Dismiss command
    // output") instead of leaving the literal string alone.
    renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={() => undefined}
        output={{
          titleKey: "slash.output.helpTitle", tone: "info",
          messages: [{ key: "slash.output.helpEntry", params: { invocation: "/status", description: "slash.output.dismiss" } }],
        }}
      />,
    );
    expect(screen.getByText("/status — slash.output.dismiss")).toBeTruthy();
  });

  it("marks an error tone for assistive technology and applies a visible error treatment", () => {
    renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={() => undefined}
        output={{ titleKey: "slash.error.title", tone: "error", messages: [{ key: "slash.error.usageUnavailable" }] }}
      />,
    );
    const panel = screen.getByTestId("slash-command-output");
    expect(panel.getAttribute("data-tone")).toBe("error");
    expect(panel.getAttribute("role")).toBe("alert");
    // Sighted users need more than `role="alert"` to tell a failed command apart from a
    // successful one — this locks in the destructive-tone classes that provide that signal.
    expect(panel.className).toContain("destructive");
  });

  it("dismisses on the close button", async () => {
    const onDismiss = vi.fn();
    const { user } = renderWithAppProviders(
      <SlashCommandOutput
        onDismiss={onDismiss}
        output={{ titleKey: "slash.output.applied", tone: "info", messages: [] }}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Dismiss command output" }));
    expect(onDismiss).toHaveBeenCalled();
  });
});
