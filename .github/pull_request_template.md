## Summary

Describe the user-visible outcome and why this change is needed.

## Related work

- Issue:
- OpenSpec change:

## Risk and compatibility

Describe affected platforms, migrations, security considerations, and rollback options.

## Validation

- [ ] Every command in the AGENTS.md「校验命令」section passes locally (verbatim flags — `lint:ci`, `clippy --all-targets -- -D warnings`, `fmt --check`)
- [ ] `npx playwright test` (when UI behavior changes)
- [ ] Conditional checks per AGENTS.md when applicable: coverage, contracts, and `openspec validate <change-name> --strict` for each active change touched

## Screenshots or diagnostics

Add redacted evidence when it helps reviewers. Do not include credentials, personal data, or unredacted logs.

## Checklist

- [ ] The change follows `AGENTS.md` and `openspec/project.md`.
- [ ] New behavior is covered by tests.
- [ ] Documentation and both service adapters are updated where applicable.
- [ ] No secrets, signing material, local databases, or sensitive logs are committed.
