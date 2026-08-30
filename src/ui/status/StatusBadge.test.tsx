// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { CheckCircle2 } from "lucide-react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { StatusBadge } from "./StatusBadge";

describe("StatusBadge", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("always renders visible text, never relying on tone alone", () => {
    render(<StatusBadge label="Blocked" tone="blocked" />);
    expect(screen.getByText("Blocked")).toBeTruthy();
  });

  it("falls back to a shape marker when no icon is given", () => {
    const { container } = render(<StatusBadge label="Running" tone="running" />);
    expect(container.querySelector("svg")).toBeNull();
    expect(container.querySelector("[aria-hidden='true'].rounded-full")).not.toBeNull();
  });

  it("renders a caller-supplied icon instead of the shape marker", () => {
    const { container } = render(<StatusBadge icon={CheckCircle2} label="Completed" tone="success" />);
    expect(container.querySelector("svg")).not.toBeNull();
    expect(container.querySelector("[aria-hidden='true'].rounded-full")).toBeNull();
  });

  it("exposes an accessible description separate from the visible label", () => {
    render(<StatusBadge description="Waiting on required approval" label="Blocked" tone="blocked" />);
    const badge = screen.getByText("Blocked").closest("span");
    const describedBy = badge?.getAttribute("aria-describedby");
    expect(describedBy).toBeTruthy();
    expect(document.getElementById(describedBy ?? "")?.textContent).toBe("Waiting on required approval");
  });

  it("applies the matching semantic tone class for every tone", () => {
    const tones = ["neutral", "running", "success", "warning", "danger", "information", "blocked", "attention"] as const;
    for (const tone of tones) {
      const { container, unmount } = render(<StatusBadge label={tone} tone={tone} />);
      expect(container.querySelector(`.ucd-status-${tone}`)).not.toBeNull();
      unmount();
    }
  });
});
