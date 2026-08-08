## Why

`multilingual-readme` requires every repository-relative link target to be identical across `README.md`, `README.zh-CN.md`, and `README.ja.md`. That rule was written when the only translated artifact was the README itself, and it holds well for command blocks, version facts, and reference links.

It no longer holds for documentation guides, because the guide sets are not translations of each other:

- The Simplified Chinese user guide has 20 chapters; the English one has 7. Thirteen have no English counterpart.
- `docs/zh/` is a 25-page Chinese architecture set with no English counterpart at all.
- No Japanese guide exists.

Under the current rule every README must carry every other language's guide links. A Chinese reader opening `README.zh-CN.md` is shown three English destinations alongside the two Chinese ones and has to work out which apply; a Japanese reader is shown five, none of which are Japanese. The parity check is enforcing sameness on the one thing that is legitimately different.

## What Changes

- Introduce a `docs-locale-guides` block marker that delimits the documentation-guide links each README offers its own readers.
- Exempt link targets inside that block from the identical-target rule, so each README routes readers only to guides written in its language.
- Keep the block itself under parity: a README missing it, or carrying more than one, fails the check. Everything outside the block — sections, commands, reference links, version facts, feature classifications — is unchanged.
- Each README's documentation section now lists only its own language. `README.ja.md` states that Japanese guides are planned rather than listing English or Chinese ones.

## Capabilities

### New Capabilities

None. This change adjusts one requirement on an existing documentation capability.

### Modified Capabilities

- `multilingual-readme`: link-target parity gains a declared exemption for locale-scoped documentation guides, and the presence of that block becomes a parity-checked structural element.

## Impact

**Runtime scope: neither.** This change touches documentation and a documentation CI script only. It does not modify desktop runtime or Web runtime behavior, React components, frontend service interfaces, Tauri adapters, Rust commands, or SQLite schema, and therefore does not affect frontend/backend isolation or any runtime adapter boundary.

Affected files: `scripts/check-readme-parity.mjs`, `scripts/check-readme-parity.node-test.mjs`, `README.md`, `README.zh-CN.md`, `README.ja.md`.

**Accepted trade-off.** English and Japanese readers lose the pointer to the Chinese guides, which are currently the more complete set. The alternative — showing every language's links in every README — is what this change removes. The English and Japanese guides are to be filled in separately; until then the gap is visible in the guides' own indexes rather than in the READMEs.

`scripts/validate-docs.mjs` continues to verify that every link inside the block resolves, so an exempt link cannot rot.
