// @vitest-environment jsdom

import { useState } from "react";
import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Accordion, type AccordionItem } from "./Accordion";

const ITEMS: AccordionItem[] = [
  { id: "alpha", header: "Alpha", content: <p>Alpha content</p> },
  { id: "beta", header: "Beta", content: <p>Beta content</p> },
  { id: "gamma", header: "Gamma", content: <p>Gamma content</p> },
];

/** A real, controlled consumer — `Accordion` takes no internal state of its own. */
function ControlledAccordion({ initialOpenIds = [] as string[], items = ITEMS }: { initialOpenIds?: string[]; items?: AccordionItem[] }) {
  const [openIds, setOpenIds] = useState(initialOpenIds);
  return <Accordion items={items} onOpenIdsChange={setOpenIds} openIds={openIds} />;
}

/** Counts its own renders so a test can prove it was never unmounted across a toggle. */
function MountCounter({ label }: { label: string }) {
  const [renderCount] = useState(() => ++MountCounter.instances);
  return <p>{label} mounted #{renderCount}</p>;
}
MountCounter.instances = 0;

describe("Accordion", () => {
  it("renders every section's header and content", () => {
    render(<ControlledAccordion initialOpenIds={["alpha"]} />);

    expect(screen.getByRole("button", { name: "Alpha" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Beta" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Gamma" })).toBeTruthy();
    expect(screen.getByText("Alpha content")).toBeTruthy();
  });

  it("wires aria-expanded, aria-controls, and aria-labelledby between each header and its content", () => {
    render(<ControlledAccordion initialOpenIds={["alpha"]} />);

    const openHeader = screen.getByRole("button", { name: "Alpha" });
    const closedHeader = screen.getByRole("button", { name: "Beta" });
    expect(openHeader.getAttribute("aria-expanded")).toBe("true");
    expect(closedHeader.getAttribute("aria-expanded")).toBe("false");

    const contentId = openHeader.getAttribute("aria-controls");
    expect(contentId).toBeTruthy();
    const content = document.getElementById(contentId ?? "");
    expect(content).toBeTruthy();
    expect(content?.getAttribute("aria-labelledby")).toBe(openHeader.getAttribute("id"));
    expect(within(content as HTMLElement).getByText("Alpha content")).toBeTruthy();
  });

  it("does not use a tablist/tabpanel role pairing", () => {
    render(<ControlledAccordion initialOpenIds={["alpha"]} />);

    expect(screen.queryByRole("tablist")).toBeNull();
    expect(screen.queryByRole("tab")).toBeNull();
    expect(screen.queryByRole("tabpanel")).toBeNull();
  });

  it("toggles a section open and closed on click", () => {
    render(<ControlledAccordion />);
    const header = screen.getByRole("button", { name: "Alpha" });
    expect(header.getAttribute("aria-expanded")).toBe("false");

    fireEvent.click(header);
    expect(header.getAttribute("aria-expanded")).toBe("true");

    fireEvent.click(header);
    expect(header.getAttribute("aria-expanded")).toBe("false");
  });

  it("supports more than one section open at the same time", () => {
    render(<ControlledAccordion />);

    fireEvent.click(screen.getByRole("button", { name: "Alpha" }));
    fireEvent.click(screen.getByRole("button", { name: "Beta" }));

    expect(screen.getByRole("button", { name: "Alpha" }).getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("button", { name: "Beta" }).getAttribute("aria-expanded")).toBe("true");
    // Opening Beta must not have closed Alpha — this is not a single-open accordion.
    expect(screen.getByText("Alpha content").closest("[hidden]")).toBeNull();
    expect(screen.getByText("Beta content").closest("[hidden]")).toBeNull();
  });

  it("activates a focused header with the keyboard, via Enter and Space", async () => {
    const user = userEvent.setup();
    render(<ControlledAccordion />);
    const header = screen.getByRole("button", { name: "Alpha" });

    header.focus();
    await user.keyboard("{Enter}");
    expect(header.getAttribute("aria-expanded")).toBe("true");

    await user.keyboard(" ");
    expect(header.getAttribute("aria-expanded")).toBe("false");
  });

  it("keeps collapsed content mounted rather than unmounting it", () => {
    const items: AccordionItem[] = [
      { id: "alpha", header: "Alpha", content: <MountCounter label="alpha" /> },
    ];
    const { rerender } = render(<Accordion items={items} onOpenIdsChange={() => {}} openIds={["alpha"]} />);
    expect(screen.getByText("alpha mounted #1")).toBeTruthy();

    // Collapse it — a lazily-mounted implementation would remove this from the DOM entirely.
    rerender(<Accordion items={items} onOpenIdsChange={() => {}} openIds={[]} />);
    const hiddenContent = document.querySelector("[hidden]");
    expect(hiddenContent?.textContent).toContain("alpha mounted #1");

    // Re-expanding must not have remounted it (the count would bump to #2 if it had).
    rerender(<Accordion items={items} onOpenIdsChange={() => {}} openIds={["alpha"]} />);
    expect(screen.getByText("alpha mounted #1")).toBeTruthy();
  });

  it("hides collapsed content with the native hidden attribute", () => {
    render(<ControlledAccordion initialOpenIds={["alpha"]} />);

    const betaHeader = screen.getByRole("button", { name: "Beta" });
    const betaContentId = betaHeader.getAttribute("aria-controls") ?? "";
    expect(document.getElementById(betaContentId)?.hasAttribute("hidden")).toBe(true);

    const alphaHeader = screen.getByRole("button", { name: "Alpha" });
    const alphaContentId = alphaHeader.getAttribute("aria-controls") ?? "";
    expect(document.getElementById(alphaContentId)?.hasAttribute("hidden")).toBe(false);
  });
});
