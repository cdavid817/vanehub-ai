// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { CopyDiagnosticsButton } from "./CopyDiagnosticsButton";

describe("CopyDiagnosticsButton", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("copies the formatted summary and shows a copied confirmation", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });
    render(
      <CopyDiagnosticsButton
        fields={[
          { label: "Version", value: "2.1.237" },
          { label: "Last checked", value: null },
        ]}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Copy diagnostics" }));
    expect(writeText).toHaveBeenCalledWith("Version: 2.1.237\nLast checked: unavailable");
    expect(await screen.findByRole("button", { name: "Diagnostics copied" })).toBeTruthy();
  });
});
