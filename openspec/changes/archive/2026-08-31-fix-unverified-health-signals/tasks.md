## 1. Rust dependency updates reach a lockfile

- [x] 1.1 Point the cargo ecosystem at the directory that holds `Cargo.lock`, keeping the existing `webview2-com` version pin and the label, schedule and grouping already configured.
- [x] 1.2 Add a check that fails when a configured ecosystem directory holds no manifest the updater can read, so the next move of a lockfile is caught here rather than by a year of silence.

## 2. A test that cannot run says so

- [x] 2.1 Replace the Playwright sidecar test's bare early `return` with a skip that names the missing prerequisite on stderr.
- [x] 2.2 Separate "the package is installed" from "a browser is installed", so the guard tests what the test actually needs.
- [x] 2.3 Keep the full-path assertions unchanged, and verify they still run where a browser exists.

## 3. Cleanup is budgeted rather than left over

- [x] 3.1 Give the isolated server test's cleanup phase a minimum budget floor, applied when the caller's deadline is already spent.
- [x] 3.2 Add a test proving a spent deadline still produces an observed forced termination and a succeeded cleanup phase.
- [x] 3.3 Add a test proving the floor does not extend a cleanup that finishes inside the caller's remaining budget.

## 4. A budget nobody is testing does not decide the result

- [x] 4.1 Give the Skill Tool process tests a wall-time they are not asserting on, so a slow `rustc` startup on a loaded runner stops failing assertions about argument literalness.
- [x] 4.2 Name the refusal code the child-ceiling test means, so a wall-time timeout can no longer satisfy it.

## 5. An assertion holds on the platform it runs on

- [x] 5.1 Compare canonical paths on both sides of the dossier export containment assertion, so Windows' extended-length prefix stops failing a check about containment.

## 6. Verification

- [x] 6.1 Run `npm run lint:ci`, `npm run test`, `npm run build`, Cargo fmt/check/clippy/panic-check/test, and `openspec validate --specs --strict`.
- [x] 6.2 Run `npm run architecture:check`.
- [x] 6.3 Record each platform actually exercised as PASSED/FAILED/BLOCKED/NOT RUN without inferring the others.
