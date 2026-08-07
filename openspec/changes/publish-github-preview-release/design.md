## Context

`.github/workflows/package.yml` already implements the full release path: a `v*` tag triggers a three-platform build matrix, then a publish job that emits `SHA256SUMS`, an SPDX SBOM, build-provenance attestations, and a GitHub Release. The `release` GitHub environment exists and restricts deployment to `v*` tags; `build-preview` exists with no restriction. `docs/release-signing.md` documents the intended sequence.

Nothing in that path has ever run. `gh run list --workflow=package.yml` and `gh release list` are both empty, and no signing secrets are configured (`gh secret list` is empty), so every package will be unsigned and un-notarized.

Two facts constrain the version scheme, both verified against `tauri-bundler` source rather than assumed:

`crates/tauri-bundler/src/bundle/windows/msi/mod.rs` — `convert_version()` aborts the build:

```rust
if !version.pre.is_empty() {
  let pre = version.pre.parse::<u64>();
  if pre.is_ok() && pre.unwrap() <= 65535 {
    return Ok(format!("{}.{}.{}.{}", version.major, version.minor, version.patch, version.pre));
  } else {
    crate::error::bail!("optional pre-release identifier in app version must be \
      numeric-only and cannot be greater than 65535 for msi target");
  }
}
```

`crates/tauri-bundler/src/bundle/windows/nsis/mod.rs` — `try_add_numeric_build_number()` ignores the pre-release identifier entirely and emits `major.minor.patch.0` for `VIProductVersion`. It never fails on it.

A repository-wide search for `version.pre` inside `tauri-bundler` matches only the MSI module, which is the negative evidence that no other bundler rejects a pre-release identifier.

`crates/tauri-bundler/src/bundle/linux/rpm.rs` passes `settings.version_string()` to `rpm::PackageBuilder::new` with no sanitization. The RPM `Version` field cannot contain a hyphen, and every semantic-versioning pre-release identifier is introduced by one. Whether `rpm-rs` rejects this or silently emits a package with an unparseable version was not determined.

## Goals / Non-Goals

**Goals:**

- Publish repeated public preview builds whose version number identifies exactly which preview a bug report refers to.
- Keep the three version declarations synchronized, preserving the existing `desktop-release-delivery` guarantee rather than weakening it to accommodate previews.
- Make the build matrix rehearsable without creating a tag.
- Give a first-time downloader on an unsigned build enough instruction to actually install and run the application.

**Non-Goals:**

- Code signing, Apple notarization, and Windows Authenticode. Deliberately deferred; the release notes disclose the consequence instead.
- The Tauri updater. `TAURI_SIGNING_PRIVATE_KEY` is already wired into the workflow environment but `tauri.conf.json` declares no updater plugin, so it is currently inert. It stays inert.
- Translating release notes. The canonical README is English and release notes follow it.
- Restoring MSI and RPM once a non-pre-release version ships. That is a separate decision once the preview program ends.
- Any change to desktop or Web runtime behavior. This change does not touch React components, frontend service interfaces, Tauri adapters, Rust commands, or SQLite.

## Decisions

### Readable pre-release identifiers, at the cost of two bundle formats

Version becomes `0.1.0-preview.1`, incrementing through `preview.2`, `rc.1`, and so on until `0.1.0`. `bundle.targets` changes from `"all"` to the explicit list `["nsis", "app", "dmg", "deb", "appimage"]`, dropping `msi` and `rpm`.

`BundleTarget` accepts either `"all"` or a list of `BundleType` (`deb`, `rpm`, `appimage`, `msi`, `nsis`, `app`, `dmg`), and targets that do not apply to the running platform are skipped, so one declared list serves all three platforms. Listing both `app` and `dmg` reproduces what `"all"` produced on macOS.

Alternatives considered:

- **Numeric-only pre-release (`0.1.0-1`, `0.1.0-2`)** keeps MSI, since `convert_version()` accepts a numeric identifier and maps it to `0.1.0.1`. Rejected: it does not survive the stated release plan. `rc.1` is not numeric, so the scheme breaks the first time a release candidate is cut, and `0.1.0-1` does not tell a bug reporter whether they are on a preview or a candidate. It also does not rescue RPM, which fails on the hyphen regardless of what follows it.
- **Keep `0.1.0` in all three manifests and put the suffix only in the tag.** No bundler is affected and `targets: "all"` survives. Rejected: `preview.1` and `preview.2` would both report `0.1.0` in the installed application, which defeats the purpose of a preview program, and it requires loosening `check-version-sync.mjs` from exact equality to prefix matching — trading a strong invariant for a weaker one.
- **Build metadata (`0.1.0+preview.1`).** Rejected: MSI applies the same numeric-only constraint to `version.build`, and build metadata is excluded from semantic-versioning precedence, so it does not order releases.

RPM is dropped rather than tested because the failure mode that matters is the silent one — an installable package whose version string cannot be compared by `dnf`. AppImage already covers RPM-based distributions.

### Detect pre-release from the tag, not from a separate input

The publish job passes `--prerelease` when the tag name contains a hyphen. Semantic versioning guarantees a pre-release identifier is introduced by a hyphen and that no other version component may contain one, so the tag alone is sufficient and there is nothing for a maintainer to forget to set. The `release` environment already restricts deployment to `v*` tags, so the tag namespace this rule interprets is constrained.

`--latest=false` is passed alongside it. GitHub does not promote a pre-release to latest, but the `--latest` default is documented as "automatic based on date and version", and pinning it removes the ambiguity.

### Fix the untagged validation path in the script, not only in the workflow

`scripts/check-version-sync.mjs` resolves the tag as `process.argv[2] ?? process.env.GITHUB_REF_NAME`. On a `workflow_dispatch` run against a branch, `package.yml` substitutes an empty argument, `argv[2]` is `undefined`, and the fallback yields the branch name — `main` — which is then compared against `v0.1.0` and fails. This is why no rehearsal is currently possible.

The fallback is corrected to apply only when `GITHUB_REF_TYPE` is `tag`. Fixing the workflow's argument passing alone would leave a script that silently reinterprets a branch name as a release tag for any future caller; fixing the script makes the invariant hold regardless of who calls it, and the workflow's conditional argument becomes redundant rather than load-bearing.

### Compare README version facts after un-escaping the badge

`check-readme-parity.mjs` extracts the visible version from the badge URL with `/badge\/version-([0-9]+\.[0-9]+\.[0-9]+)-/i` and compares it to the `docs-fact:project-version` marker, which in turn is compared to `package.json`. Shields.io requires a literal hyphen in a static badge to be escaped as `--`, so the badge URL becomes `badge/version-0.1.0--preview.1-blue.svg`. The current pattern captures `0.1.0` from that string and reports a mismatch against `0.1.0-preview.1`.

The pattern is extended to capture the escaped pre-release segment, and the captured value is un-escaped (`--` to `-`) before comparison. Comparing the un-escaped form rather than escaping the manifest value keeps the shields.io encoding rule confined to one place.

This must land in the same commit as the README edits. `npm run docs:check` runs in CI (`.github/workflows/ci.yml:156`), so a version bump without the pattern change, or a pattern change without the badge updates, breaks the branch.

### Ship release notes from a tracked file, prepended to generated notes

Preview-specific guidance lives in `.github/PREVIEW_RELEASE_NOTES.md` so it is reviewable, diffable, and cannot drift from the workflow that consumes it. The publish job prepends it to the auto-generated pull-request summary.

`gh release create --help` documents the prepend behavior for `--notes` specifically and does not name `--notes-file`, but both populate the same field. In `pkg/cmd/release/create/create.go`, `--notes-file` is read into `opts.Body`, and generated notes are appended to whatever body is already present:

```go
if opts.Body == "" {
  params["body"] = generatedNotes.Body
} else {
  params["body"] = fmt.Sprintf("%s\n%s", opts.Body, generatedNotes.Body)
}
```

The preview guidance therefore lands above the generated pull-request summary, which is the intended order. No concatenation step is needed.

The notes file stays version-independent so it does not need editing per preview: it refers to assets by format rather than by filename, and the `xattr` path is the installed application path, not a download path.

## Risks / Trade-offs

- **The first tagged run is still the first tagged run.** → The rehearsal fix makes a `workflow_dispatch` run exercise validate, all three builds, and artifact upload. Only the publish job remains untested, because it is gated on `github.ref_type == 'tag'`. That residual exposure is accepted; a failed publish leaves a pushed tag with no release, which is recoverable by deleting the tag and re-tagging.
- **MSI and RPM consumers lose their format.** → Windows NSIS installs per-user without administrator rights and is Tauri's recommended Windows installer; AppImage runs on RPM-based distributions. Both exclusions are documented as accepted limitations under the `native-app-packaging` requirement that already forbids silently treating an unsupported combination as supported.
- **macOS remains the weakest download.** An un-notarized application from a browser download carries a quarantine attribute and reports being damaged. → The release notes carry the `xattr -cr` instruction. This is a disclosure, not a fix, and conversion on macOS will be poor until an Apple Developer certificate is provisioned.
- **Linux binaries inherit the runner's glibc.** Building on `ubuntu-latest` produces binaries that will not start on older distributions. → Out of scope for this change; recorded as an open question rather than silently shipped.
- **The main branch advertises a pre-release version.** The README badge on the default branch will read `0.1.0-preview.1`. → Intended. The badge links to `package.json` and reflects it accurately.
- **`--notes-file` composition with `--generate-notes` is unverified.** → Rehearsal step with a documented fallback, above.

## Migration Plan

1. Land the workflow validation fix first, on its own, so a rehearsal becomes possible before anything else changes.
2. Run `workflow_dispatch` on `main` and confirm all three matrix jobs upload artifacts.
3. Land the version bump, bundle target list, parity pattern, README updates, and release notes file together — the parity check couples them.
4. Verify locally that a Windows package builds and installs, which is the only way to confirm the NSIS path end to end with a pre-release version.
5. Run `workflow_dispatch` again and install the downloaded artifacts on real machines.
6. Tag `v0.1.0-preview.1` and push.

Rollback: delete the tag and the GitHub Release. Because no updater is configured, no installed client polls for or auto-applies a release, so a withdrawn preview cannot propagate to users who already installed it — they simply keep the build they have.

### Cover Intel Macs in the preview matrix

`macos-x64` joins the build matrix on the `macos-15-intel` runner. `package:macos:x64` already existed in `package.json` and only the matrix entry was missing, so an Intel Mac user would otherwise have had no download at all — the worst outcome for a public preview, worse than the unsigned warning.

The runner label matters here: `macos-13` has been retired, so `macos-15-intel` is the available Intel image. The alternative was cross-compiling `x86_64-apple-darwin` on an arm64 runner, which avoids depending on an Intel runner label but ships a binary that no job ever executed on its target architecture. For a preview whose purpose is to collect real installation feedback, a natively built package is worth the extra job.

## Open Questions

- Which minimum Linux distribution should preview builds support? This decides whether the Linux job pins an older runner image or whether the release notes state a minimum glibc.
- Does the preview program need a feedback channel beyond the existing issue templates — for example a discussion category, which `gh release create --discussion-category` could attach automatically?
