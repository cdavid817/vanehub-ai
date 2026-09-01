// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { DangerZone, SettingsRow } from "./page-parts";

describe("SettingsRow's mutation/error support (task 12.10)", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders exactly as before when no mutation/error props are passed", () => {
    render(<SettingsRow title="Language">field</SettingsRow>);
    expect(screen.getByText("Language")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("shows pending state only for this row while its mutation is in flight", () => {
    render(<SettingsRow mutation={{ targetKey: "applicationLanguage", pending: true }} title="Language">field</SettingsRow>);
    expect(screen.getByText(/saving|pending/i)).toBeTruthy();
  });

  it("shows a row-level save-failure message with a retry action, scoped to this row alone", () => {
    const onRetry = vi.fn();
    render(
      <SettingsRow
        mutation={{ targetKey: "applicationLanguage", pending: false, error: { kind: "error", message: "Could not save language.", retryable: true } }}
        onRetryMutation={onRetry}
        title="Language"
      >
        field
      </SettingsRow>,
    );
    expect(screen.getByRole("alert").textContent).toContain("Could not save language.");
    screen.getByRole("button", { name: "Retry" }).click();
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it("shows a field-validation error independently of any save mutation", () => {
    render(<SettingsRow errorMessage="Must not be empty." title="Language">field</SettingsRow>);
    expect(screen.getByRole("alert").textContent).toContain("Must not be empty.");
  });
});

describe("DangerZone (task 12.14)", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders its title, description, and children", () => {
    render(
      <DangerZone description="This cannot be undone." title="Reset to defaults">
        <button type="button">Reset</button>
      </DangerZone>,
    );
    expect(screen.getByText("Reset to defaults")).toBeTruthy();
    expect(screen.getByText("This cannot be undone.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Reset" })).toBeTruthy();
  });

  it("renders without a description when none is given", () => {
    render(
      <DangerZone title="Erase all data">
        <button type="button">Erase</button>
      </DangerZone>,
    );
    expect(screen.getByText("Erase all data")).toBeTruthy();
  });
});
