## 1. Carve the exemption into the parity check

- [x] 1.1 Add the `docs-locale-guides` block marker recognition to `scripts/check-readme-parity.mjs` and strip those blocks before collecting link targets
- [x] 1.2 Collect the block occurrences as a parity-compared array so a README that drops or duplicates the block fails
- [x] 1.3 Fail the canonical README when it does not carry exactly one block
- [x] 1.4 Add a unit test asserting that guide links differing inside the block are accepted
- [x] 1.5 Add a unit test asserting that a translation dropping its block is reported
- [x] 1.6 Update the shared fixture so its sample links no longer reference a deleted file

## 2. Scope each README to its own language

- [x] 2.1 `README.md`: list only the English user guide and Developer Guide inside the block
- [x] 2.2 `README.zh-CN.md`: list only the Chinese user guide and the Chinese architecture set inside the block
- [x] 2.3 `README.ja.md`: state that Japanese guides are planned, without listing another language's guides
- [x] 2.4 Keep reference links, build commands, and the toolchain note outside the block so they stay under parity

## 3. Verify

- [x] 3.1 `npm run docs:check`
- [x] 3.2 `npm run docs:test`
- [x] 3.3 `npm run lint:ci`
- [ ] 3.4 `openspec validate scope-readme-guides-per-language --strict`
- [ ] 3.5 Confirm the Documentation job passes on the pull request
