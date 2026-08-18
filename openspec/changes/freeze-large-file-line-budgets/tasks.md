## 1. Native budget registry and counter

- [x] 1.1 Add a line-counting helper to `src-tauri/tests/architecture.rs` that returns physical lines for a single path and the aggregate for a subtree, reusing the existing `rust_files()` traversal rather than adding a second one
- [x] 1.2 Add the path-budget registry with the five native entries from design.md, each carrying the owning decomposition work so the diagnostic can name it
- [x] 1.3 Add the subtree-budget registry with the two native entries from design.md
- [x] 1.4 Implement the budget test so a path over budget, and a subtree over budget, each fail with the path, the measured count, the recorded budget, and the owning decomposition work, using the established `[ARCH-NATIVE-*]` diagnostic format

## 2. Native budget rule fixtures

- [x] 2.1 Add an accepting fixture: a path under its budget and a subtree under its budget both pass
- [x] 2.2 Add a rejecting fixture for a path over its budget, asserting the diagnostic carries the measured count and the recorded budget
- [x] 2.3 Add a rejecting fixture for a subtree over its budget, asserting the diagnostic names the subtree rather than an individual file
- [x] 2.4 Add a fixture proving a registered path that does not exist is treated as satisfied while its subtree budget still applies

## 3. Frontend per-file budgets

- [x] 3.1 Remove the `src/services/coordination-runtime.ts` entry from the `eslint.config.js` technical-debt block — the file no longer exists
- [x] 3.2 Replace the block's `"max-lines": "off"` with per-file `max-lines` overrides carrying the eight budgets from design.md, keeping `skipBlankLines: false, skipComments: false` so the counting matches the global rule
- [x] 3.3 Update the block comment to state that entries are budgets that may only be lowered without justification, and that raising one requires a stated reason
- [x] 3.4 Confirm each recorded budget matches what ESLint measures, adjusting for the trailing-newline difference noted in design.md, so the gate passes on an unmodified tree

## 4. Frontend subtree budget

- [x] 4.1 Add a `lineBudget` rule id and repair text to `scripts/architecture/rules.mjs` alongside the existing frontend rules
- [x] 4.2 Add the frontend subtree-budget registry and a `wc -l`-equivalent line counter to `scripts/architecture/frontend-rules.mjs`, reusing `productionFiles()` for the walk
- [x] 4.3 Emit the budget diagnostic from `checkFrontendArchitecture()` so `node scripts/architecture/check.mjs` fails when the subtree exceeds its budget
- [x] 4.4 Add accepting and rejecting fixtures to `scripts/architecture/frontend-rules.node-test.mjs`, including one proving the counter matches `wc -l` on sources with and without a trailing newline

## 5. Verification

- [x] 5.1 `cargo test --manifest-path src-tauri/Cargo.toml --test architecture` passes, including the new budget test and its fixtures
- [x] 5.2 `node --test scripts/architecture/*.node-test.mjs` and `node scripts/architecture/check.mjs` pass on an unmodified tree
- [x] 5.3 `npm run lint:ci` passes on an unmodified tree
- [x] 5.4 Prove the gate bites: temporarily append lines past a budget on each of the three homes, confirm each fails with the expected diagnostic, then revert
- [x] 5.5 `npm run architecture:check`, `npm run test`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `openspec validate freeze-large-file-line-budgets --strict` all pass
