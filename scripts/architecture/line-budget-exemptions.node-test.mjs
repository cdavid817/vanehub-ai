import assert from "node:assert/strict";
import test from "node:test";
import { LEGACY_LINE_BUDGET_EXEMPTIONS } from "../../eslint.config.js";

// eslint.config.js's own comment already says entries may not be added — this test makes that
// enforceable rather than only reviewable. redesign-unified-workbench-ui's task 1.6 requires every
// new or modified production TS/TSX file to stay within the repository's 300-line limit; the one
// way that requirement could quietly break is a new file (most importantly under the brand-new
// src/ui/ tree this change creates) picking up an entry here instead of being held to the global
// max-lines rule in eslint.config.js.

test("the legacy line-budget exemption list has not grown past its recorded size", () => {
  assert.equal(LEGACY_LINE_BUDGET_EXEMPTIONS.length, 6);
});

test("no exemption entry covers a file under src/ui/", () => {
  const uiEntries = LEGACY_LINE_BUDGET_EXEMPTIONS.filter(([file]) => file.startsWith("src/ui/"));
  assert.deepEqual(uiEntries, []);
});

test("every entry is a [path, budget] pair with a positive integer budget", () => {
  for (const entry of LEGACY_LINE_BUDGET_EXEMPTIONS) {
    assert.equal(entry.length, 2);
    const [file, budget] = entry;
    assert.equal(typeof file, "string");
    assert.ok(Number.isInteger(budget) && budget > 0, `${file} has a non-positive-integer budget`);
  }
});
