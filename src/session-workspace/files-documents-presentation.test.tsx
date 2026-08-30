/** @vitest-environment jsdom */
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import { ContentSearchPanel } from "./content-search-panel";
import { QuickOpenDialog } from "./quick-open-dialog";
import { LITERAL_COLOR, PALETTE_COLOR, TOKEN_RULE_SAMPLES } from "./visual-token-rules";

/**
 * How the Files and Documents surfaces answer the keyboard, and what they are allowed to look like.
 *
 * The keyboard half is written as a parity suite over both search dialogs rather than as two sets
 * of cases. Their source says they behave the same way on purpose — "a reader who has learned one
 * surface should not have to learn the other" — and a claim like that is only worth anything if
 * something checks it. Two separate suites would let one drift and stay green.
 *
 * The style half scans sources. A rendering test can only see the states somebody thought to
 * render, and the thing that goes wrong here is a control added in a hurry with a literal colour in
 * it, which looks correct in whichever theme its author had open.
 */

const CONTEXT = { availability: "available" as const, rootName: "project", reason: null };

interface SearchSurface {
  readonly name: string;
  /**
   * Sets up the service double so a query returns exactly these rows, in order, and hands back the
   * spy. The spy is the only way to tell the typed query's answer from the one the surface issued
   * for itself on open — and until the typed one has landed, the highlight can still be reset.
   */
  readonly arrange: (rows: readonly string[]) => ReturnType<typeof vi.spyOn>;
  /**
   * Whether the surface refuses to search until something is typed.
   *
   * Quick Open lists the workspace for an empty query and content search deliberately does not, so
   * typing into both would issue a second search in one of them for no reason. See `openWithRows`.
   */
  readonly needsQuery: boolean;
  readonly open: (onClose: () => void, onSelect: (chosen: unknown) => void) => void;
  /** The row a selection callback stands for, so both surfaces can be asserted the same way. */
  readonly rowOf: (chosen: unknown) => string;
}

const SURFACES: readonly SearchSurface[] = [
  {
    arrange: (rows) =>
      vi.spyOn(agentService, "searchWorkspacePaths").mockResolvedValue({
        generation: 1,
        coverage: { state: "complete" },
        matches: rows.map((path) => ({ kind: "file", name: path, path })),
      }),
    name: "Quick Open",
    needsQuery: false,
    open: (onClose, onSelect) => {
      renderWithAppProviders(
        <QuickOpenDialog isOpen onClose={onClose} onSelect={onSelect} sessionId="session-1" />,
      );
    },
    rowOf: (chosen) => (chosen as { path: string }).path,
  },
  {
    arrange: (rows) => {
      vi.spyOn(agentService, "cancelWorkspaceSearch").mockResolvedValue(true);
      return vi.spyOn(agentService, "searchWorkspaceContent").mockResolvedValue({
        generation: 1,
        coverage: { state: "complete" },
        matches: rows.map((path, index) => ({
          column: 1,
          line: index + 1,
          path,
          snippet: `match in ${path}`,
          snippetTruncated: false,
        })),
      });
    },
    name: "Search in files",
    needsQuery: true,
    open: (onClose, onSelect) => {
      renderWithAppProviders(
        <ContentSearchPanel isOpen onClose={onClose} onSelect={onSelect} sessionId="session-1" />,
      );
    },
    rowOf: (chosen) => (chosen as { path: string }).path,
  },
];

const ROWS = ["a.rs", "b.rs", "c.rs"];

beforeAll(async () => {
  await activateAppLanguage("en");
});

beforeEach(() => {
  vi.spyOn(agentService, "listSessionDirectory").mockResolvedValue({
    context: CONTEXT,
    coverage: { state: "complete" },
    items: [],
    nextCursor: null,
    path: "",
    truncated: false,
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

const TYPED = "rs";

/**
 * Opens a surface with rows on screen, one search behind them, and no effects left pending.
 *
 * Both surfaces send the highlight back to the top whenever a result set arrives, which is right —
 * the rows underneath it changed. The trap is that this runs in an effect queued by the same commit
 * that renders those rows, and React runs passive effects after the commit. So there is a window
 * where the rows are on screen and the reset has not happened yet, and a test that starts pressing
 * keys the moment it sees rows gets its keystrokes undone by an effect from before it pressed them.
 *
 * Two things close it. Nothing is typed unless the surface refuses to search without a query, so
 * there is one answer rather than two; and the pending effects are drained before any key is
 * pressed. Without the drain this suite passed on its own and failed in a full run, where the extra
 * load was enough to leave the effect queued.
 */
async function openWithRows(surface: SearchSurface) {
  const search = surface.arrange(ROWS);
  const onClose = vi.fn();
  const onSelect = vi.fn();
  surface.open(onClose, onSelect);
  const input = screen.getByRole("combobox");
  if (surface.needsQuery) fireEvent.change(input, { target: { value: TYPED } });
  await waitFor(
    () => {
      expect(search).toHaveBeenCalledTimes(1);
      expect(screen.getAllByRole("option")).toHaveLength(ROWS.length);
    },
    { timeout: 4000 },
  );
  await act(async () => {});
  return { input, onClose, onSelect };
}

/** The row Enter would take, read off the screen rather than inferred from the keys pressed. */
function highlightedRow(): string {
  const active = screen
    .getAllByRole("option")
    .find((option) => option.getAttribute("aria-selected") === "true");
  return active?.textContent ?? "";
}

describe.each(SURFACES)("$name answers the keyboard", (surface) => {
  it("moves with the arrows and opens what is highlighted", async () => {
    const { input, onSelect } = await openWithRows(surface);

    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowUp" });
    // Checked before Enter, so what the reader can see and what Enter takes are asserted to be the
    // same row rather than assumed to be.
    await waitFor(() => expect(highlightedRow()).toContain(ROWS[1]));

    fireEvent.keyDown(input, { key: "Enter" });
    expect(surface.rowOf(onSelect.mock.calls[0]?.[0])).toBe(ROWS[1]);
  });

  it("stops at both ends rather than wrapping", async () => {
    const { input, onSelect } = await openWithRows(surface);

    fireEvent.keyDown(input, { key: "ArrowUp" });
    // Wrapping is right for a find inside one file and wrong for a result list: there, the reader
    // is walking a set they can see the ends of, and a jump to the far end reads as a lost place.
    await waitFor(() => expect(highlightedRow()).toContain(ROWS[0]));
    fireEvent.keyDown(input, { key: "Enter" });
    expect(surface.rowOf(onSelect.mock.calls[0]?.[0])).toBe(ROWS[0]);

    onSelect.mockClear();
    for (let step = 0; step < ROWS.length + 2; step += 1) {
      fireEvent.keyDown(input, { key: "ArrowDown" });
    }
    await waitFor(() => expect(highlightedRow()).toContain(ROWS[ROWS.length - 1]));
    fireEvent.keyDown(input, { key: "Enter" });
    expect(surface.rowOf(onSelect.mock.calls[0]?.[0])).toBe(ROWS[ROWS.length - 1]);
  });

  it("keeps the caret in the input so typing can continue", async () => {
    const { input } = await openWithRows(surface);

    fireEvent.keyDown(input, { key: "ArrowDown" });
    // The whole point of both surfaces is refining a query while looking at the answers. A list that
    // took focus on arrow-down would end that, and every keystroke after it would go nowhere.
    expect(document.activeElement).toBe(input);
  });

  it("marks exactly one row as the one Enter would take", async () => {
    const { input } = await openWithRows(surface);
    fireEvent.keyDown(input, { key: "ArrowDown" });
    await waitFor(() => expect(highlightedRow()).toContain(ROWS[1]));

    const selected = screen
      .getAllByRole("option")
      .filter((option) => option.getAttribute("aria-selected") === "true");
    expect(selected).toHaveLength(1);
  });

  it("announces the row Enter would take, without moving focus to it", async () => {
    const { input } = await openWithRows(surface);

    fireEvent.keyDown(input, { key: "ArrowDown" });
    await waitFor(() => expect(highlightedRow()).toContain(ROWS[1]));

    // The pairing that makes "focus stays in the input while the highlight moves" announceable.
    // Without it the DOM focus never moves, so a screen reader has nothing to read out on the
    // down arrow: the reader hears the query they typed and nothing about what Enter would do.
    const named = input.getAttribute("aria-activedescendant");
    expect(named).toBeTruthy();
    const highlighted = screen
      .getAllByRole("option")
      .find((option) => option.getAttribute("aria-selected") === "true");
    expect(highlighted?.id).toBe(named);
    expect(document.activeElement).toBe(input);
  });

  it("puts the options directly inside the listbox", async () => {
    await openWithRows(surface);

    // A button carrying `role="option"` inside an `li` puts a listitem between the listbox and
    // its options, which the accessibility tree does not allow — and the interactive descendant is
    // what breaks the activedescendant pairing above.
    for (const option of screen.getAllByRole("option")) {
      expect(option.parentElement?.getAttribute("role")).toBe("listbox");
      expect(option.querySelector("button")).toBeNull();
    }
  });

  it("says the list is closed when there is nothing in it", async () => {
    surface.arrange([]);
    surface.open(vi.fn(), vi.fn());
    const input = screen.getByRole("combobox");
    if (surface.needsQuery) fireEvent.change(input, { target: { value: TYPED } });

    // An expanded combobox with no options tells a reader there is something to arrow through.
    await waitFor(() => expect(input.getAttribute("aria-expanded")).toBe("false"), { timeout: 4000 });
    expect(input.getAttribute("aria-activedescendant")).toBeNull();
    // And the empty message is not inside the listbox, where it would announce itself as one more
    // thing to choose from.
    expect(screen.queryAllByRole("option")).toHaveLength(0);
  });

  it("closes on Escape without choosing anything", async () => {
    const { input, onClose, onSelect } = await openWithRows(surface);

    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Escape" });

    expect(onClose).toHaveBeenCalled();
    expect(onSelect).not.toHaveBeenCalled();
  });
});

/**
 * The same two patterns 14.5 holds over every console surface, read from the same place.
 *
 * Two copies would drift, and the copy that drifts is always the one with fewer readers — at which
 * point Files and Documents are held to a rule the rest of the console is not, and nothing on
 * either side says so.
 */
const PALETTE = PALETTE_COLOR;
const LITERAL_VALUE = LITERAL_COLOR;

/** Utilities that occupy space. A state change that touches one of these moves its neighbours. */
const DIMENSION = [
  /^-?[mp][trblxy]?-/,
  /^(?:min-|max-)?[hw]-/,
  /^gap(?:-[xy])?-/,
  /^border(?:-[trblxy])?(?:-\d+)?$/,
  /^text-(?:xs|sm|base|lg|\d?xl)$/,
  /^(?:leading|tracking|space-[xy]|inset|top|right|bottom|left|translate|scale)-/,
];

const SURFACE_MODULES = /^(?:files?-|document|quick-open|content-search|use-file|use-content|use-workspace-file)/;

function surfaceSources(): { name: string; source: string }[] {
  const directory = dirname(fileURLToPath(import.meta.url));
  return readdirSync(directory)
    .filter((name) => /\.tsx?$/.test(name) && !name.includes(".test.") && SURFACE_MODULES.test(name))
    .map((name) => ({ name, source: readFileSync(join(directory, name), "utf8") }));
}

describe("the Files and Documents surfaces look like the rest of the application", () => {
  it("scans the modules it claims to, with patterns that match a real violation", () => {
    const names = surfaceSources().map((entry) => entry.name);
    expect(names).toContain("files-tab.tsx");
    expect(names).toContain("documents-tab.tsx");
    expect(names).toContain("quick-open-dialog.tsx");

    // Proved against strings rather than trusted. A typo in either pattern leaves a check that
    // passes because it matches nothing, which is the failure a source scan hides best.
    expect(PALETTE.test(TOKEN_RULE_SAMPLES.paletteMatches)).toBe(true);
    expect(PALETTE.test(TOKEN_RULE_SAMPLES.paletteRejects)).toBe(false);
    expect(LITERAL_VALUE.test(TOKEN_RULE_SAMPLES.literalMatches)).toBe(true);
    expect(LITERAL_VALUE.test(TOKEN_RULE_SAMPLES.literalRejects)).toBe(false);
  });

  it("takes every colour from the shared tokens", () => {
    const offenders = surfaceSources()
      .filter(({ source }) => PALETTE.test(source) || LITERAL_VALUE.test(source))
      .map(({ name }) => name);

    // A literal colour is correct in exactly the theme its author had open and wrong in the other.
    // Nothing about it looks wrong to the person who wrote it, which is why this is a scan.
    expect(offenders).toEqual([]);
  });

  it("changes only appearance when a row becomes the highlighted one", async () => {
    const surface = SURFACES[0];
    if (!surface) throw new Error("no search surface to check");
    await openWithRows(surface);
    fireEvent.keyDown(screen.getByRole("combobox"), { key: "ArrowDown" });
    await waitFor(() => expect(highlightedRow()).toContain(ROWS[1]));

    const options = screen.getAllByRole("option");
    const active = options.find((option) => option.getAttribute("aria-selected") === "true");
    const idle = options.find((option) => option.getAttribute("aria-selected") !== "true");
    if (!active || !idle) throw new Error("expected one highlighted row and one that is not");

    const idleClasses = new Set(idle.className.split(/\s+/));
    const added = active.className.split(/\s+/).filter((token) => token && !idleClasses.has(token));

    // Non-vacuous: the highlight has to be visible at all before "it is only a colour" means
    // anything.
    expect(added.length).toBeGreaterThan(0);
    // A highlight that also changed padding would shift every row below it, so walking the list
    // with the arrow keys would make the list move under the reader.
    expect(added.filter((token) => DIMENSION.some((pattern) => pattern.test(token)))).toEqual([]);
  });
});
