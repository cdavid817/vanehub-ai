import assert from "node:assert/strict";
import test from "node:test";
import {
  hasDocumentedSymbol,
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
