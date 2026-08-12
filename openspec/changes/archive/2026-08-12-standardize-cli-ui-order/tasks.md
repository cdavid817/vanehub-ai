## 1. Shared ordering policy

- [x] 1.1 Add a stable frontend Agent priority utility with settings and create-session order definitions
- [x] 1.2 Align mirrored managed CLI and configurable-Agent collections with the requested priority
- [x] 1.3 Add utility tests covering full, subset, and unknown-Agent stable ordering

## 2. Settings and create-session integration

- [x] 2.1 Apply the shared settings priority to CLI management, parameters, configuration, policy, Prompt Hook, Skill, and IM Agent lists
- [x] 2.2 Apply the five-CLI create-session priority without moving OnePiece out of its native group
- [x] 2.3 Add component and Playwright regression coverage for settings and create-session ordering

## 3. Verification

- [x] 3.1 Run focused ordering unit, component, and Playwright tests
- [x] 3.2 Run the frontend, Rust, coverage, contract, build, and UI validation commands required by `AGENTS.md`
- [x] 3.3 Run `openspec validate standardize-cli-ui-order --strict` and `openspec validate --specs --strict`
