## Summary

Describe the user-visible outcome and why this change is needed.

## Related work

- Issue:
- OpenSpec change:

## Risk and compatibility

Describe affected platforms, migrations, security considerations, and rollback options.

## Validation

- [ ] `npm run lint`
- [ ] `npm run test`
- [ ] `npm run contracts:check`
- [ ] `npm run build`
- [ ] `npx playwright test` (when UI behavior changes)
- [ ] Rust fmt, check, Clippy, and tests (when native code changes)
- [ ] Strict OpenSpec validation

## Screenshots or diagnostics

Add redacted evidence when it helps reviewers. Do not include credentials, personal data, or unredacted logs.

## Checklist

- [ ] The change follows `AGENTS.md` and `openspec/project.md`.
- [ ] New behavior is covered by tests.
- [ ] Documentation and both service adapters are updated where applicable.
- [ ] No secrets, signing material, local databases, or sensitive logs are committed.
