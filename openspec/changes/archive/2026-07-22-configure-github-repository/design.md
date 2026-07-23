## Context

The public GitHub repository needs a single operational contract spanning pull-request validation, default-branch governance, dependency and code security, public collaboration, and release delivery. The repository already has npm, Cargo, Vitest, OpenSpec, and Playwright commands, but turning them into required checks also exposes platform-specific formatting, path, window, and E2E stability defects. The existing package workflow produces transient artifacts without a complete GitHub Release integrity chain.

This is repository and delivery infrastructure. It affects both Web and Tauri validation, but it does not introduce new frontend service methods, direct React-to-Tauri calls, database behavior, or runtime adapter divergence.

## Goals / Non-Goals

**Goals:**

- Establish green, least-privilege CI for every pull request and push to `main`.
- Protect `main` only after required check names exist and pass.
- Automate dependency maintenance, vulnerable-dependency review, CodeQL analysis, and immutable Action maintenance.
- Publish cross-platform desktop artifacts with synchronized versions, checksums, an SPDX SBOM, attestations, and generated release notes.
- Provide the ownership, security, contribution, issue, pull-request, support, and labeling conventions expected of a public repository.
- Consolidate the full implementation and its validation evidence under one OpenSpec change.

**Non-Goals:**

- Store or fabricate code-signing, Apple Developer, or notarization credentials.
- Add an in-application updater or change application APIs, SQLite schemas, service boundaries, or adapter contracts.
- Replace npm, Cargo, Vitest, Playwright, OpenSpec, or GitHub-native security features.
- Require a second maintainer's approval in a repository that currently has one maintainer.

## Decisions

- Use independent GitHub Actions jobs for frontend, OpenSpec, Rust, native platform checks, and Playwright. This gives focused required checks and preserves later diagnostics when an unrelated job fails; a single sequential job was rejected because early failures hide evidence.
- Use Node.js with `npm ci`, stable Rust with rustfmt and Clippy, explicit Tauri Linux packages, and Chromium-only Playwright matching the committed project configuration. Existing package managers and test frameworks remain the source of truth.
- Run full Vitest plus contract checks and strict OpenSpec validation. Upload Playwright reports only on failure, with missing reports ignored so diagnostics cannot mask the original result.
- Repair deterministic baseline defects exposed by multi-platform CI in their owning layer: Rust retains native path/window/process concerns, while E2E configuration and assertions remain browser-test concerns. React service and Web/Tauri adapter boundaries remain unchanged.
- Use a GitHub repository ruleset instead of legacy branch protection. Require known CI checks, pull requests, and resolved conversations; reject deletion and force pushes; retain zero required approvals until multiple maintainers can review each other.
- Pin every external Action to a full commit SHA and let Dependabot update npm, Cargo, and GitHub Actions dependencies. Workflow tokens are read-only by default, with narrowly scoped permissions for security analysis, attestations, and releases.
- Split release building from publication. Platform jobs upload bundles; one release job validates versions, collects successful artifacts, generates checksums and SPDX output, attests artifacts, and publishes exactly one GitHub Release.
- Scope signing and notarization inputs to a protected `release` environment. Unsigned builds remain explicit until maintainers provision real encrypted secrets and platform configuration.
- Use GitHub issue forms, CODEOWNERS, path labels, and generated release notes instead of adding third-party governance bots.

## Risks / Trade-offs

- [Required checks are activated before workflows register their names] → Push and observe the workflows before enabling the ruleset, then read back the rule configuration.
- [Platform runners expose baseline defects] → Fix only deterministic cross-platform behavior and retain targeted tests without weakening assertions.
- [GitHub-hosted runner or Tauri package changes break native validation] → Install declared prerequisites explicitly and keep platform checks separate.
- [Pinned Action SHAs reduce readability] → Retain version comments and use Dependabot to maintain pins.
- [A platform release build fails] → Block the publication job so a partial artifact set is not presented as a complete release.
- [Signing credentials are unavailable] → Document exact environment inputs and never persist placeholder secrets.
- [Remote GitHub settings are not represented by Git history] → Verify settings through read-only API calls and retain their intended behavior in OpenSpec.

## Migration Plan

1. Add and run the complete CI configuration locally and on the feature branch.
2. Add security, community, ownership, labeling, and release configuration.
3. Push the branch and verify all registered GitHub checks are green.
4. Apply and read back repository security, merge, Actions, environment, and default-branch ruleset settings.
5. Merge through the protected pull-request path; provision real signing credentials separately when available.

Rollback disables the new ruleset and workflows, reverts the repository configuration commit, and removes only newly created labels or environments that are not in use.

## Open Questions

- Windows signing provider and Apple Developer credentials remain maintainer-supplied deployment inputs.
- Additional release architectures may require native runners or an explicit cross-compilation environment after the first release dry run.
