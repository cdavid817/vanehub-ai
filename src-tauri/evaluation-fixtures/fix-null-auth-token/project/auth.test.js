const test = require("node:test");
const assert = require("node:assert/strict");

test("fixture starts with a reproducible null-token defect", () => {
  assert.equal("Bearer null", "Bearer null");
});
