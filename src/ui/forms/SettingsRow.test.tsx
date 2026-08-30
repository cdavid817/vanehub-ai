// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { SettingsRow } from "./SettingsRow";

describe("SettingsRow", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders title, description, and the control", () => {
    render(
      <SettingsRow description="Used throughout the interface" title="Language">
        <select><option>English</option></select>
      </SettingsRow>,
    );
    expect(screen.getByText("Language")).toBeTruthy();
    expect(screen.getByText("Used throughout the interface")).toBeTruthy();
    expect(screen.getByRole("combobox")).toBeTruthy();
  });

  it("shows per-row mutation feedback without a page-wide busy flag", () => {
    render(
      <SettingsRow mutation={{ targetKey: "language", pending: true }} title="Language">
        <select><option>English</option></select>
      </SettingsRow>,
    );
    expect(screen.getByRole("status").textContent).toContain("Saving");
    expect((screen.getByRole("combobox") as HTMLSelectElement).disabled).toBe(false);
  });

  it("surfaces a retryable mutation error with a working retry action", () => {
    const retry = vi.fn();
    render(
      <SettingsRow
        mutation={{ targetKey: "language", pending: false, error: { kind: "error", message: "Could not save.", retryable: true } }}
        onRetryMutation={retry}
        title="Language"
      >
        <select><option>English</option></select>
      </SettingsRow>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledOnce();
  });

  it("renders a field validation error tight against the row", () => {
    render(
      <SettingsRow errorMessage="Choose a supported language." title="Language">
        <select><option>English</option></select>
      </SettingsRow>,
    );
    expect(screen.getByRole("alert").textContent).toContain("Choose a supported language.");
  });
});
