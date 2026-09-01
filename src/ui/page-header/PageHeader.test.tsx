// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { KeyRound } from "lucide-react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { PageHeader } from "./PageHeader";

describe("PageHeader", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders title, breadcrumb, description, status summary, and the primary action", () => {
    render(
      <PageHeader
        breadcrumb={<span>Runs</span>}
        description="All active and recently completed agent runs."
        primaryAction={<button type="button">New Run</button>}
        statusSummary={<span>3 attention</span>}
        title="Mission Control"
      />,
    );
    expect(screen.getByRole("heading", { name: "Mission Control" })).toBeTruthy();
    expect(screen.getByText("Runs")).toBeTruthy();
    expect(screen.getByText("All active and recently completed agent runs.")).toBeTruthy();
    expect(screen.getByText("3 attention")).toBeTruthy();
    expect(screen.getByRole("button", { name: "New Run" })).toBeTruthy();
  });

  it("bounds a long description instead of letting it grow the header freely", () => {
    render(<PageHeader description="A very long description that keeps going." title="Mission Control" />);
    expect(screen.getByText("A very long description that keeps going.").className).toContain("line-clamp-2");
  });

  it("renders the icon badge only when one is supplied", () => {
    const { rerender } = render(<PageHeader title="Mission Control" />);
    expect(document.querySelector("svg")).toBeNull();

    rerender(<PageHeader icon={KeyRound} title="Mission Control" />);
    expect(document.querySelector("svg")).toBeTruthy();
  });

  it("omits the More menu when no items are supplied", () => {
    render(<PageHeader title="Mission Control" />);
    expect(screen.queryByRole("button", { name: "More actions" })).toBeNull();
  });

  it("renders a working More menu when items are supplied", () => {
    const onSelect = vi.fn();
    render(<PageHeader moreMenuItems={[{ id: "export", label: "Export", onSelect }]} title="Mission Control" />);
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Export" }));
    expect(onSelect).toHaveBeenCalledOnce();
  });
});
