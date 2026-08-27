## 1. Runner Discovery

- [x] 1.1 Add native Runner discovery tests proving Local remains available when session-bound SSH lookup fails
- [x] 1.2 Make optional Runner discovery fail-soft and project a bounded warning without launching a process
- [x] 1.3 Update Runner selector tests and localized status text so partial discovery is not presented as global unavailability
- [x] 1.4 Omit unsupported placeholder Runners from the native catalog and cover final descriptor validation with a regression test

## 2. Live Run Feedback

- [x] 2.1 Add elapsed-time presenter tests for advancing active Runs and frozen terminal Runs
- [x] 2.2 Compute active elapsed time against the current clock while preserving canonical terminal duration
- [x] 2.3 Derive seat-attributed member activity states and render compact localized roster feedback
- [x] 2.4 Close the canonical Run from managed CLI success, failure, and cancellation paths and add a duplicate-terminal regression test

## 3. Member Streaming

- [x] 3.1 Add stream subscription tests for newly created seat messages whose events race the message cache
- [x] 3.2 Reconcile unknown message rows without dropping incremental thinking, tool, or token output
- [x] 3.3 Verify Tauri and Web/mock adapters preserve stable seat attribution and the shared stream contract

## 4. Verification And Specification Sync

- [x] 4.1 Run focused Rust, Vitest, and Playwright coverage for Runner and multi-Agent runtime feedback
- [x] 4.2 Run the repository validation commands from AGENTS.md, including desktop runtime checks for the affected boundary
- [x] 4.3 Validate the OpenSpec change strictly and synchronize its delta requirements into the main specifications
- [x] 4.4 Re-run focused Runner tests, native checks, frontend checks, and strict OpenSpec validation after the catalog regression fix
- [x] 4.5 Run focused and repository validation after the managed CLI terminal-state fix, then synchronize the requirement into the main specification
