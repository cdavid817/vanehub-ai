## Context

The public GitHub repository currently has no branch protection or rulesets, no Dependabot or code-scanning automation, no release history, and a 42% community profile. Secret scanning and push protection are enabled, and the default Actions token is read-only. The local CI change establishes frontend, Rust, and Playwright jobs, but the current branch has rustfmt drift and the E2E suite has failures that must be understood before checks become mandatory. The existing packaging workflow uploads transient workflow artifacts but does not create a GitHub Release or establish binary provenance.

## Goals / Non-Goals

**Goals:**

- Establish a green, least-privilege CI baseline that enforces all repository validation contracts.
- Protect `main` with pull-request and status-check rules after the checks exist and pass.
- Automate dependency maintenance, vulnerable-dependency review, CodeQL analysis, and immutable Action references.
- Publish cross-platform desktop artifacts through GitHub Releases with version checks, checksums, an SBOM, and attestations.
- Provide ownership, contribution, security-reporting, issue, pull-request, support, and release-note conventions for a public project.
- Apply safe repository settings through authenticated GitHub APIs and verify the resulting remote state.

**Non-Goals:**

- Invent or store code-signing certificates, Apple credentials, or other maintainer secrets.
- Enable an in-application updater, because the updater plugin and update manifest contract are not part of this change.
- Add paid services or replace npm, Cargo, Vitest, Playwright, or OpenSpec.
- Change application architecture, runtime behavior, database schemas, or service adapters.

## Decisions

- Keep the existing CI change as the foundation and add repository-hardening behavior in a separate OpenSpec change. This avoids conflating basic CI introduction with public repository governance and release security.
- Repair rustfmt mechanically. Diagnose E2E failures before changing application behavior; improve retry and retained diagnostics only where it does not hide failures.
- Run full Vitest and pinned OpenSpec 1.6.0 validation in CI. Use explicit workflow permissions, concurrency cancellation, and job timeouts. A single monolithic job was rejected because independent checks provide clearer required statuses.
- Use GitHub repository rulesets rather than a legacy branch-protection rule. The ruleset targets the default branch, blocks deletion and non-fast-forward updates, requires pull requests and resolved conversations, and requires known CI checks. Review approval remains zero while the repository has one maintainer; CODEOWNERS documents ownership without making self-review impossible.
- Use native GitHub security features and pinned Actions: Dependabot for npm, Cargo, and Actions; dependency review for manifest changes; CodeQL for JavaScript/TypeScript and Rust; secret scanning remains enabled. General `npm audit` is not a merge gate because registry-wide audit noise is less actionable than dependency review and Dependabot alerts.
- Pin every Action to a full commit SHA and let Dependabot maintain those pins. Workflow-level tokens remain read-only; security analysis and release jobs receive only the additional permissions they need.
- Split packaging from publishing. Matrix jobs build and upload per-platform bundles; one release job downloads them, generates checksums and an SPDX JSON SBOM, creates provenance/SBOM attestations, and publishes one GitHub Release. This avoids concurrent release creation from matrix jobs.
- Expose supported Tauri/macOS signing environment variables from a protected `release` environment. Missing credentials must not be represented as configured; unsigned builds remain explicit until the maintainer installs secrets and platform-specific signing configuration.
- Prefer GitHub issue forms and generated release notes over additional bots. Automated labeling is limited to path-based ownership areas.

## Risks / Trade-offs

- [Activating required checks before workflows exist can block merges] → Apply repository rules only after configuration is committed, pushed, and check names are verified; otherwise stage the ruleset in evaluation or defer activation.
- [Current E2E failures can keep the branch red] → Preserve failure evidence, separate baseline defects from workflow defects, and do not weaken assertions to make CI pass.
- [Cross-compilation for Linux ARM64 and Windows ARM64 may lack host tooling] → Validate each matrix target and keep failed architectures from producing misleading releases.
- [Pinned Action SHAs are harder to read] → Retain version comments and configure Dependabot for `github-actions`.
- [Release secrets are unavailable in the worktree] → Configure the protected environment and documented secret names, but treat signing/notarization as pending until real credentials are supplied.
- [Repository settings are external state] → Read back every mutated setting and report any plan/permission limitation.

## Migration Plan

1. Repair and expand local CI, then run all checks.
2. Add dependency, code-scanning, community, and release files and validate workflow syntax.
3. Commit and push the feature branch so GitHub can register workflow check names.
4. Enable security settings, labels, merge settings, the release environment, and the default-branch ruleset through GitHub APIs.
5. Open or prepare a pull request and observe the first CI run before merging.
6. Add real signing/notarization secrets through GitHub's encrypted secret UI or API outside the repository.

Rollback consists of disabling the ruleset and security workflows, reverting the repository configuration commit, and deleting only the newly created repository labels/environment if they are not in use.

## Open Questions

- Windows code signing provider and Apple Developer credentials remain maintainer-supplied deployment inputs.
- Linux ARM64 may require a native ARM runner or an explicit cross-compilation container after the first release dry run.
