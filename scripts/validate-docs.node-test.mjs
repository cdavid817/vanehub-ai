import assert from "node:assert/strict";
import test from "node:test";
import {
  boundedContextDrift,
  headingIds,
  documentedBoundedContexts,
  hasDocumentedSymbol,
  normalizeHeadingId,
  unclosableEmphasis,
  validateNativeBoundaryContent,
} from "./validate-docs.mjs";

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
