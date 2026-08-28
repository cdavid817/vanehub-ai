/** @vitest-environment jsdom */
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * Files and Documents read. They do not write, and this is what holds that.
 *
 * The rule is easy to state and easy to lose: every one of the surfaces this change added — a tree,
 * a preview, two searches, a toolbar, a document viewer — is one keystroke of scope creep away from
 * "and you can rename it here". A guard that reads the sources is the only kind that survives
 * somebody adding a control in a hurry, because a behavioural test can only fail for the control
 * somebody thought to write a test for.
 *
 * Scoped to the panels this change owns. Changes and Review legitimately mutate — reverting a hunk
 * is their whole purpose — and a guard that swept them in would be a guard somebody has to weaken,
 * which is a guard that stops meaning anything.
 */

/** The Files and Documents surfaces, by the prefixes they are named with. */
const READ_ONLY_MODULES = /^(files?-|document|quick-open|content-search|use-file|use-content|use-workspace-file)/;

/**
 * Service calls that change a file, and the DOM affordances that offer to.
 *
 * Matched as call and attribute forms rather than as words, so a comment explaining why something
 * is absent does not trip the check. That mistake has been made twice in this change already, and
 * both times the tempting "fix" was to delete the explanation.
 */
const MUTATIONS: readonly { readonly pattern: RegExp; readonly what: string }[] = [
  { pattern: /\.\s*writeSessionFile\s*\(/, what: "writes a file" },
  { pattern: /\.\s*saveSessionFile\s*\(/, what: "saves a file" },
  { pattern: /\.\s*deleteSessionFile\s*\(/, what: "deletes a file" },
  { pattern: /\.\s*renameSessionFile\s*\(/, what: "renames a file" },
  { pattern: /\.\s*createSessionFile\s*\(/, what: "creates a file" },
  { pattern: /\.\s*createSessionDirectory\s*\(/, what: "creates a directory" },
  { pattern: /\.\s*revertReviewChange\s*\(/, what: "reverts a change" },
  { pattern: /contentEditable/, what: "makes content editable" },
  { pattern: /<textarea/i, what: "offers a text area" },
];

function readOnlySources(): { name: string; source: string }[] {
  const directory = dirname(fileURLToPath(import.meta.url));
  return readdirSync(directory)
    .filter((name) => /\.tsx?$/.test(name) && !name.includes(".test.") && READ_ONLY_MODULES.test(name))
    .map((name) => ({ name, source: readFileSync(join(directory, name), "utf8") }));
}

describe("the Files and Documents surfaces are read-only", () => {
  it("covers the modules it claims to", () => {
    const names = readOnlySources().map((entry) => entry.name);

    // A guard that matched nothing would pass forever. These four are the surfaces most likely to
    // grow an edit control, so their presence is what makes the rest of this suite mean something.
    expect(names).toContain("files-tab.tsx");
    expect(names).toContain("documents-tab.tsx");
    expect(names).toContain("file-preview.tsx");
    expect(names).toContain("file-preview-toolbar.tsx");
  });

  it("uses patterns that match a real mutation", () => {
    // Proved against the one surface that legitimately mutates. Without this, a typo in any of the
    // patterns would leave a guard that passes because it matches nothing at all — the failure mode
    // a source-scanning test is most prone to and least likely to reveal.
    const reviewCenter = readFileSync(
      join(dirname(fileURLToPath(import.meta.url)), "review-center.tsx"),
      "utf8",
    );

    expect(MUTATIONS.some(({ pattern }) => pattern.test(reviewCenter))).toBe(true);
  });

  it("calls nothing that changes a file", () => {
    const offenders = readOnlySources().flatMap(({ name, source }) =>
      MUTATIONS.filter(({ pattern }) => pattern.test(source)).map(
        ({ what }) => `${name} ${what}`,
      ),
    );

    expect(offenders).toEqual([]);
  });

  it("binds no input to file content", () => {
    const offenders = readOnlySources()
      .filter(({ source }) =>
        // An input whose value is the file's own text is an editor with the save button missing,
        // and the save button is the easy part to add later. The find and go-to-line boxes are
        // bound to their own state, never to content.
        /value=\{[^}]*\b(?:content|file\.content|preview\.content)\b/.test(source),
      )
      .map(({ name }) => name);

    expect(offenders).toEqual([]);
  });
});
