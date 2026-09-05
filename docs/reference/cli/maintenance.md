# Updating the CLI parameter registry

The canonical registry is
`src-tauri/src/contexts/tooling/cli_parameters/catalog/catalog.v2.json`. Everything else —
the frontend contract, the settings page, the argv a launch emits, and the published matrix — is
derived from it. Change the registry, then regenerate; never the other way round.

## When to run this

- a managed CLI ships a release that adds, renames, or removes a flag
- a flag's accepted values change, or a value becomes model- or version-dependent
- a parameter is deprecated
- the six-month re-audit falls due

## 1. Read the official source, then check the binary

Both, in that order, because they disagree. The 2026-08-23 audit found two cases where they did:

- `claude --help` on 2.1.237 does not list `--advisor`, and its `--effort` rejection message omits
  `ultracode`. The published reference lists both, and the binary accepts both — an unknown option
  produces `error: unknown option`, and `--advisor` does not, while an unknown effort produces
  `Unknown --effort value` and `ultracode` does not.
- `codex-cli` 0.149.0 rejects `--ask-for-approval untrusted` outright, although the registry still
  offered it.

So: read the vendor's published reference for intent and version gates, and probe the installed
binary for what it actually accepts. A flag the binary rejects is wrong in the registry no matter
what the page says.

Useful probes, none of which make a model call:

```bash
claude --help                     # and: claude --<flag> <value> -p "" to see whether it parses
codex --help && codex exec --help # subcommand flags live under the subcommand
gemini --help
opencode --help && opencode run --help
agy --help
```

An authentication failure means the flag parsed. A usage error means it did not. That is the
discriminator; do not read a 403 as a rejected flag.

## 2. Edit the registry

Each parameter carries an `audit` block:

| Field | Meaning |
| --- | --- |
| `sourceId` | stable identifier for the source, not a URL |
| `sourceUrl` | the page that was read |
| `reviewedAt` | the date of *this* review, not the date the parameter was added |
| `reviewedState` | which artefact was read and in what state, including the binary version |
| `verification` | `verified`, `repository-verified`, or `pending-review` |
| `note` | what the review established, especially anything surprising |

`verified` requires a source the vendor publishes — its documentation, or its own binary's help and
argument-rejection behaviour. `repository-verified` means only something in this repository confirms
it, which is never sufficient on its own. `pending-review` means the source did not settle it;
carry the parameter forward but do not present it as audited.

Never infer. If a value cannot be confirmed, omit it or mark the parameter `pending-review`.

Approval, auto-approval, permission, sandbox and dangerous-bypass parameters stay `policy-governed`;
prompts, session ids, output protocol tokens and credentials never enter the registry at all.

## 3. Regenerate and verify

```bash
npm run contracts:generate     # frontend catalog + published matrix
npm run contracts:check        # fails on drift, on a missing locale key, on registry parity
cargo test --workspace         # registry validation, argv equivalence, provider builders
npm run test                   # frontend adapters, draft engine, page
```

Adding an option also needs its localization keys in **every** registered locale;
`src/contracts/cli-parameter-localization.test.ts` fails otherwise, in every locale at once.

## 4. Desktop smoke, when argv changed

```bash
npm run test:desktop:build
npm run test:desktop:smoke
```

`tests/desktop/specs/domain-cli-tooling.e2e.mjs` exercises list, preview, save, reset and the
optimistic-concurrency rejections against a real client. It is where a registry change that breaks
argv shows up as something other than a unit-test diff.

## 5. Record the review

Update `src/contracts/fixtures/cli-parameter-source-audit.json` with the review date, the source
URLs and the binary versions the check ran against. The contract test asserts that both maps cover
every managed CLI, so a partial audit cannot be recorded as a complete one.
