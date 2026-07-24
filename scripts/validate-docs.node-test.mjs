import assert from "node:assert/strict";
import test from "node:test";
import {
  hasDocumentedSymbol,
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
