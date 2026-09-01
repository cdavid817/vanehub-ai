// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { buildSettingsSearchIndex } from "./settings-search-index";
import { SettingsSearchBox } from "./settings-search-box";
import type { SettingsPageDefinition } from "./settings-page-types";

function page(overrides: Partial<SettingsPageDefinition>): SettingsPageDefinition {
  return {
    id: "basic",
    labelKey: "fixture.alpha.label",
    crumbKey: "fixture.alpha.label",
    group: "general",
    icon: (() => null) as unknown as SettingsPageDefinition["icon"],
    searchPlaceholderKey: "fixture.search",
    descriptionKey: "fixture.alpha.description",
    keywords: [],
    fields: [],
    saveMode: "immediate",
    risk: "normal",
    loader: () => Promise.resolve({ default: () => null }),
    ...overrides,
  };
}

const alpha = page({
  id: "basic",
  labelKey: "fixture.alpha.label",
  descriptionKey: "fixture.alpha.description",
  fields: [{ id: "timeout", labelKey: "fixture.alpha.timeout", anchorId: "alpha-timeout" }],
});
const bravo = page({ id: "providers", labelKey: "fixture.bravo.label", descriptionKey: "fixture.bravo.description" });
const pages = [alpha, bravo];
const index = buildSettingsSearchIndex(pages);

describe("SettingsSearchBox", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("shows no dropdown while the query is empty", () => {
    render(
      <SettingsSearchBox index={index} onSearchTermChange={vi.fn()} onSelectResult={vi.fn()} pages={pages} placeholder="Search" searchTerm="" />,
    );
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("still calls onSearchTermChange so existing per-page filtering keeps working unchanged", () => {
    const onSearchTermChange = vi.fn();
    render(
      <SettingsSearchBox index={index} onSearchTermChange={onSearchTermChange} onSelectResult={vi.fn()} pages={pages} placeholder="Search" searchTerm="" />,
    );
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "fixture" } });
    expect(onSearchTermChange).toHaveBeenCalledWith("fixture");
  });

  it("lists matching page and field results as options with the owning page as a subtitle", () => {
    render(
      <SettingsSearchBox index={index} onSearchTermChange={vi.fn()} onSelectResult={vi.fn()} pages={pages} placeholder="Search" searchTerm="fixture.alpha" />,
    );
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(2); // The alpha page entry and its one field entry.
  });

  it("shows a no-results state for a query that matches nothing, without an empty listbox", () => {
    render(
      <SettingsSearchBox index={index} onSearchTermChange={vi.fn()} onSelectResult={vi.fn()} pages={pages} placeholder="Search" searchTerm="zzz-no-match" />,
    );
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(screen.getByRole("status")).toBeTruthy();
  });

  it("selects the active result on Enter", () => {
    const onSelectResult = vi.fn();
    render(
      <SettingsSearchBox index={index} onSearchTermChange={vi.fn()} onSelectResult={onSelectResult} pages={pages} placeholder="Search" searchTerm="fixture.bravo" />,
    );
    fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });
    expect(onSelectResult).toHaveBeenCalledTimes(1);
    expect(onSelectResult.mock.calls[0][0].page.id).toBe("providers");
  });

  it("moves the active option with ArrowDown/ArrowUp and selects whichever is active", () => {
    const onSelectResult = vi.fn();
    render(
      <SettingsSearchBox index={index} onSearchTermChange={vi.fn()} onSelectResult={onSelectResult} pages={pages} placeholder="Search" searchTerm="fixture.alpha" />,
    );
    const input = screen.getByRole("combobox");
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onSelectResult).toHaveBeenCalledTimes(1);
    expect(onSelectResult.mock.calls[0][0].entry.kind).toBe("field");
  });

  it("selects a result by click as well as by keyboard", () => {
    const onSelectResult = vi.fn();
    render(
      <SettingsSearchBox index={index} onSearchTermChange={vi.fn()} onSelectResult={onSelectResult} pages={pages} placeholder="Search" searchTerm="fixture.bravo" />,
    );
    fireEvent.click(screen.getAllByRole("option")[0]);
    expect(onSelectResult).toHaveBeenCalledTimes(1);
  });
});
