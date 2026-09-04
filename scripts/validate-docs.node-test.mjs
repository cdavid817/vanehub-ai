import assert from "node:assert/strict";
import test from "node:test";
import {
  boundedContextDrift,
  chapterBoundedContexts,
  chapterStatedContextTotal,
  documentedNpmScripts,
  headingIds,
  documentedBoundedContexts,
  hasDocumentedSymbol,
  normalizeHeadingId,
  unclosableEmphasis,
  unreachableDocuments,
  validateNativeBoundaryContent,
  staleSecurityVersions,
  staleVersionExamples,
  summaryTargets,
  duplicateTableTargets,
} from "./validate-docs.mjs";

const reachabilityGraph = {
  "SUMMARY.md": ["chapter.md"],
  "chapter.md": ["nested.md"],
  "nested.md": [],
  "orphan.md": [],
  "island-a.md": ["island-b.md"],
  "island-b.md": ["island-a.md"],
};
const reachabilityLinks = (file) => reachabilityGraph[file] ?? [];

test("treats a document the roots reach transitively as reachable", () => {
  assert.deepEqual(
    unreachableDocuments(["SUMMARY.md"], ["chapter.md", "nested.md"], reachabilityLinks),
    [],
  );
});

test("reports a document no root reaches", () => {
  assert.deepEqual(
    unreachableDocuments(["SUMMARY.md"], ["chapter.md", "orphan.md"], reachabilityLinks),
    ["orphan.md"],
  );
});

test("reports both halves of a pair that only links to itself", () => {
  assert.deepEqual(
    unreachableDocuments(["SUMMARY.md"], ["island-a.md", "island-b.md"], reachabilityLinks),
    ["island-a.md", "island-b.md"],
  );
});

test("does not report a root itself, even when nothing links to it", () => {
  assert.deepEqual(unreachableDocuments(["SUMMARY.md"], ["SUMMARY.md"], reachabilityLinks), []);
});

test("terminates on a cycle that a root does reach", () => {
  const cyclic = (file) => ({ ...reachabilityGraph, "SUMMARY.md": ["island-a.md"] })[file] ?? [];
  assert.deepEqual(
    unreachableDocuments(["SUMMARY.md"], ["island-a.md", "island-b.md"], cyclic),
    [],
  );
});

const boundary = {
  path: "src/example.rs",
  moduleDoc: true,
  symbols: ["DocumentedContract"],
};

test("accepts selected symbols with contiguous Rust documentation", () => {
  const content = `//! Example boundary.

#[derive(Clone)]
/// Explains ownership and invariants.
pub(crate) struct DocumentedContract;
`;
  assert.equal(hasDocumentedSymbol(content, "DocumentedContract"), true);
  assert.deepEqual(validateNativeBoundaryContent(boundary, content), []);
});

test("reports a selected symbol that disappears", () => {
  const errors = validateNativeBoundaryContent(boundary, "//! Example boundary.\n");
  assert.deepEqual(errors, [
    'Native documentation boundary symbol is missing: "src/example.rs#DocumentedContract".',
  ]);
});

test("reports a selected symbol whose Rust documentation is removed", () => {
  const content = `//! Example boundary.

#[derive(Clone)]
pub(crate) struct DocumentedContract;
`;
  const errors = validateNativeBoundaryContent(boundary, content);
  assert.deepEqual(errors, [
    'Native documentation boundary symbol lacks Rust documentation: "src/example.rs#DocumentedContract".',
  ]);
});

test("does not accept a detached comment as symbol documentation", () => {
  const content = `//! Example boundary.

/// Detached documentation.

pub(crate) struct DocumentedContract;
`;
  assert.equal(hasDocumentedSymbol(content, "DocumentedContract"), false);
});

test("reports bold that cannot close after a full-width colon", () => {
  assert.match(unclosableEmphasis("- **已交付：**正常创建会话对话框中的席位分配。"), /已交付/);
});

test("reports bold that re-pairs around the wrong words", () => {
  // Renders as one span covering the whole sentence instead of just the answer.
  assert.notEqual(unclosableEmphasis("**不能。**它检索的是**记忆**，不索引仓库文件。"), null);
});

test("accepts the same sentences with the punctuation moved outside", () => {
  assert.equal(unclosableEmphasis("- **已交付**：正常创建会话对话框中的席位分配。"), null);
  assert.equal(unclosableEmphasis("**不能**。它检索的是**记忆**，不索引仓库文件。"), null);
});

test("accepts bold that legitimately opens on a bracket or closes before a dash", () => {
  assert.equal(unclosableEmphasis("- **「只读」并不禁止一切**——读文件与写记忆照常放行。"), null);
  assert.equal(unclosableEmphasis("行为见（**交接解析**）的五条防御。"), null);
});

test("accepts an English span whose colon is followed by a space", () => {
  assert.equal(unclosableEmphasis("- **Delivered:** CLI management and sessions."), null);
});

test("ignores asterisks inside a code span", () => {
  assert.equal(unclosableEmphasis("`glob` 用于限定文件集（如 `**/*.rs`），**默认工作区根**。"), null);
});

const standardsTable = [
  "### Bounded contexts",
  "",
  "This table is the complete map.",
  "",
  "| Context | Ownership |",
  "| --- | --- |",
  "| `agent_runtime` | Agents |",
  "| `work_board` | Work items |",
  "",
  "- Every new rule MUST have one owning context.",
  "",
  "### Target module layout",
  "",
  "| `not_a_context` | belongs to a later section |",
].join("\r\n");

test("reads the bounded-context table from CRLF standards without spilling into later sections", () => {
  assert.deepEqual(documentedBoundedContexts(standardsTable), ["agent_runtime", "work_board"]);
});

test("reports a table that cannot be located rather than silently passing", () => {
  assert.deepEqual(documentedBoundedContexts("# Standards\r\n\r\nNo table here.\r\n"), []);
});

test("reports contexts missing from the table and rows missing from disk", () => {
  assert.deepEqual(
    boundedContextDrift(["agent_runtime", "ghost"], ["agent_runtime", "work_board"]),
    { stale: ["ghost"], undocumented: ["work_board"] },
  );
});

test("accepts a table that matches the directories exactly", () => {
  assert.deepEqual(
    boundedContextDrift(["agent_runtime", "work_board"], ["agent_runtime", "work_board"]),
    { stale: [], undocumented: [] },
  );
});

const contextMapChapter = [
  "# Native bounded contexts",
  "",
  "### Agent execution",
  "",
  "| Context | Owns |",
  "| --- | --- |",
  "| `agent_runtime` | Provider invocation |",
  "",
  "### Desktop",
  "",
  "| Context | Owns |",
  "| --- | --- |",
  "| `work_board` | Work items |",
  "",
  "## Facades",
  "",
  "| `agent_runtime` | Repeated in the second table |",
  "",
  "Prose naming `browser_automation` does not document it.",
].join("\n");

test("collects context rows across every table in the chapter and dedupes repeats", () => {
  assert.deepEqual(chapterBoundedContexts(contextMapChapter), ["agent_runtime", "work_board"]);
});

test("does not count a context that the chapter only names in prose", () => {
  assert.ok(!chapterBoundedContexts(contextMapChapter).includes("browser_automation"));
});

test("reports no rows rather than passing when the chapter has no tables", () => {
  assert.deepEqual(chapterBoundedContexts("# Map\n\nNothing tabulated here.\n"), []);
});

test("keeps a hyphen inside a word and drops punctuation around it", () => {
  // The defect this check exists for: a link guessed `planagent` where mdBook keeps the hyphen.
  assert.equal(normalizeHeadingId("Plan-Agent loop"), "plan-agent-loop");
  assert.equal(
    normalizeHeadingId("Fidelity: why some nodes do not expand"),
    "fidelity-why-some-nodes-do-not-expand",
  );
});

test("keeps CJK characters and lowercases only ASCII", () => {
  assert.equal(
    normalizeHeadingId("\u7b2c 24 \u7ae0 OnePiece \u539f\u751f Plan-Agent \u5faa\u73af"),
    "\u7b2c-24-\u7ae0-onepiece-\u539f\u751f-plan-agent-\u5faa\u73af",
  );
});

test("suffixes repeated headings the way mdBook does", () => {
  const ids = headingIds("<memory>", ["## Limits", "", "## Limits", "", "## Limits"].join("\n"));
  assert.deepEqual([...ids], ["limits", "limits-1", "limits-2"]);
});

test("ignores headings inside fenced code and strips inline markup", () => {
  const ids = headingIds("<memory>", [
    "## **Bold** heading",
    "",
    "```text",
    "## not a heading",
    "```",
    "",
    "## A [linked](target.md) heading",
  ].join("\n"));
  assert.deepEqual([...ids].sort(), ["a-linked-heading", "bold-heading"]);
});

// The chapters said "24 contexts" while the tree held 27 and their own tables held 27 rows.
// The table check could not catch it: a number in prose is not a table row.
test("reads the total a context-map chapter states in Chinese", () => {
  const chapter = [
    "# Native 限界上下文",
    "",
    "`src-tauri/src/contexts/` 下当前有 **27 个上下文**。下表是完整地图。",
    "",
    "| `agent_runtime` | 描述 |",
  ].join("\r\n");
  assert.equal(chapterStatedContextTotal(chapter), 27);
});

test("reads the total a context-map chapter states in English", () => {
  const chapter = [
    "# Native bounded contexts",
    "",
    "`src-tauri/src/contexts/` currently holds **27 contexts**. The table below is the complete map.",
  ].join("\n");
  assert.equal(chapterStatedContextTotal(chapter), 27);
});

test("returns no total when the chapter routes to the table instead of counting", () => {
  const chapter = [
    "# Native bounded contexts",
    "",
    "`src-tauri/src/contexts/` holds one directory per context; the table below is the complete map.",
  ].join("\n");
  assert.equal(chapterStatedContextTotal(chapter), null);
});

test("does not mistake an emphasised number elsewhere in the chapter for the total", () => {
  const chapter = [
    "This table covers only the **8 contexts** whose schemas are most stable.",
    "A workspace may hold **27 contexts** worth of state.",
  ].join("\n");
  assert.equal(chapterStatedContextTotal(chapter), null);
});

// All three READMEs documented `npm run tauri -- dev` against a manifest defining only
// `tauri:dev`. Parity compares command blocks across languages, so a command that is wrong
// in every language at once passes it.
test("names the script npm would run, not the arguments after the separator", () => {
  assert.deepEqual(documentedNpmScripts("npm run tauri -- dev"), ["tauri"]);
  assert.deepEqual(documentedNpmScripts("npm run dev -- --host 127.0.0.1"), ["dev"]);
});

test("collects colon-separated script names and dedupes repeats", () => {
  const content = [
    "```powershell",
    "npm run tauri:dev",
    "```",
    "Then run `npm run docs:check` and `npm run docs:check` again.",
  ].join("\n");
  assert.deepEqual(documentedNpmScripts(content), ["docs:check", "tauri:dev"]);
});

test("finds no scripts in a document that runs none", () => {
  assert.deepEqual(documentedNpmScripts("Install with `npm ci`, then open the app."), []);
});

// The security policy once promised fixes for 0.1.x while the manifest shipped 1.4.0. The
// policy is version-free now, so any concrete support-table version is drift by definition.
test("flags a hardcoded version row in the security policy", () => {
  assert.deepEqual(staleSecurityVersions("| Version | Supported |\n| 0.1.x | Yes |"), ["0.1.x"]);
});

test("accepts a version-free security policy", () => {
  assert.deepEqual(
    staleSecurityVersions("Security fixes target the latest published release line and `main`."),
    [],
  );
});

// The bug form shipped `placeholder: v0.1.0 or commit SHA` long after that version was gone.
test("flags a fixed version example in an issue-form placeholder", () => {
  assert.deepEqual(staleVersionExamples("      placeholder: v0.1.0 or commit SHA"), ["v0.1.0"]);
});

test("accepts a placeholder that describes where to find the value", () => {
  assert.deepEqual(
    staleVersionExamples("      placeholder: version shown in Settings → About, or a commit SHA"),
    [],
  );
});

test("version numbers outside placeholders do not count as stale examples", () => {
  assert.deepEqual(staleVersionExamples("      description: since release 1.0.0 this field exists"), []);
});

// The two user-guide books must offer the same chapters in the same order.
test("summary targets keep order and duplicates for exact parity comparison", () => {
  const summary = "- [A](a.md)\n  - [B](sub/b.md)\n- [A again](a.md)";
  assert.deepEqual(summaryTargets(summary), ["a.md", "sub/b.md", "a.md"]);
});

// Both developer-guide indexes once carried two rows for one file, and a user-guide index
// presented one merged chapter as two feature rows.
test("flags one target linked from two rows of the same table", () => {
  const table = [
    "| Doc | Covers |",
    "| --- | --- |",
    "| [One](target.md) | first |",
    "| [Two](target.md) | second |",
  ].join("\n");
  assert.deepEqual(duplicateTableTargets(table), ["target.md"]);
});

test("the same target in two different tables is not a duplicate row", () => {
  const content = [
    "| Doc | Covers |",
    "| --- | --- |",
    "| [One](target.md) | first |",
    "",
    "| Doc | Covers |",
    "| --- | --- |",
    "| [One](target.md) | again, in its own table |",
  ].join("\n");
  assert.deepEqual(duplicateTableTargets(content), []);
});
