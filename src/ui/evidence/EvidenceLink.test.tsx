// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router";
import { activateAppLanguage } from "../../i18n";
import { EvidenceLink } from "./EvidenceLink";

describe("EvidenceLink", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("links to the authoritative page instead of rendering evidence inline", () => {
    render(
      <MemoryRouter>
        <EvidenceLink availability="available" label="Open Diff" to="/sessions/abc/changes" />
      </MemoryRouter>,
    );
    const link = screen.getByRole("link", { name: /Open Diff/ });
    expect(link.getAttribute("href")).toBe("/sessions/abc/changes");
  });

  it("does not render a navigable link when the target is unavailable, and explains why", () => {
    render(
      <MemoryRouter>
        <EvidenceLink availability="unavailable" label="Open Diff" reason="The session was deleted." to="/sessions/abc/changes" />
      </MemoryRouter>,
    );
    expect(screen.queryByRole("link")).toBeNull();
    expect(screen.getByText("Not available")).toBeTruthy();
    expect(screen.getByText("The session was deleted.")).toBeTruthy();
  });

  it("distinguishes restricted from unavailable", () => {
    render(
      <MemoryRouter>
        <EvidenceLink availability="restricted" label="Open Diff" to="/sessions/abc/changes" />
      </MemoryRouter>,
    );
    expect(screen.getByText("Restricted")).toBeTruthy();
  });

  it("copies a reference value rather than raw evidence content", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });
    render(
      <MemoryRouter>
        <EvidenceLink availability="available" copyValue="session://abc/changes" label="Open Diff" to="/sessions/abc/changes" />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Copy link" }));
    expect(writeText).toHaveBeenCalledWith("session://abc/changes");
    expect(await screen.findByRole("button", { name: "Link copied" })).toBeTruthy();
  });

  it("gives the copy action a visible focus ring, matching every other interactive src/ui/ control", () => {
    render(
      <MemoryRouter>
        <EvidenceLink availability="available" copyValue="session://abc/changes" label="Open Diff" to="/sessions/abc/changes" />
      </MemoryRouter>,
    );
    expect(screen.getByRole("button", { name: "Copy link" }).className).toContain("ucd-focus-ring");
  });
});
