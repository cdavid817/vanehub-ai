// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../i18n";
import { ProjectsDestination } from "./projects-destination";

describe("ProjectsDestination", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("shows an honest under-construction state rather than fabricated content", () => {
    render(<ProjectsDestination />);
    expect(screen.getByText("Projects & Workspaces is under construction")).toBeTruthy();
    expect(screen.getByText(/aggregate your existing projects/)).toBeTruthy();
  });
});
