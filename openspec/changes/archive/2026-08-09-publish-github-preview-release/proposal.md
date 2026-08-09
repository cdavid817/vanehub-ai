## Why

The release pipeline in `.github/workflows/package.yml` has never executed once: `gh run list --workflow=package.yml` and `gh release list` are both empty. Its only untagged rehearsal path is broken, so the first real execution would be a tag push that publishes directly to a public repository with no way to rehearse first.

The project also needs to ship repeated public preview builds (`preview.1`, `preview.2`, `rc.1`) before `0.1.0`. The current pipeline cannot express a preview: it publishes every tag as a full latest release, and the Windows MSI bundler rejects a semantic-versioning pre-release identifier outright, so a preview version number cannot even be built.

## What Changes

- Adopt semantic-versioning pre-release identifiers (`0.1.0-preview.1`) as the release version for preview builds, kept synchronized across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
- **BREAKING** for distributable formats: restrict the Tauri bundle target set so preview and release builds no longer produce Windows MSI or Linux RPM packages. `tauri-bundler` aborts the MSI build on a non-numeric pre-release identifier, and the RPM `Version` field cannot contain the hyphen every pre-release identifier carries. Windows ships the NSIS installer; Linux ships `.deb` and AppImage.
- Repair the release workflow's untagged validation path so a manual `workflow_dispatch` run can rehearse the full build matrix without creating a tag.
- Publish pre-release tags as GitHub pre-releases rather than latest releases, so a preview never displaces a stable download.
- Publish release notes that state the packages are unsigned and un-notarized, and that carry per-platform installation guidance for the resulting Gatekeeper and SmartScreen prompts.
- Accept pre-release version identifiers in the README parity check, and route README readers to downloadable releases; no README currently links to the Releases page.

## Capabilities

### New Capabilities

None. This change adjusts requirements on existing release, packaging, and documentation capabilities.

### Modified Capabilities

- `desktop-release-delivery`: version synchronization must accept pre-release identifiers; pre-release tags must publish as GitHub pre-releases; the workflow must offer a rehearsal path that validates without a tag; unsigned distribution must be disclosed with actionable installation guidance instead of a bare statement.
- `native-app-packaging`: the bundle target set becomes an explicit declared list rather than every target the bundler supports, and the excluded formats must be documented as an accepted limitation rather than silently dropped.
- `multilingual-readme`: the parity check must treat a pre-release version as a valid canonical manifest fact, and each README must route readers to published downloads.

## Impact

**Runtime scope: neither.** This change touches build configuration, release automation, and documentation only. It does not modify desktop runtime or Web runtime behavior, React components, frontend service interfaces, Tauri adapters, Rust commands, or SQLite schema, and therefore does not affect frontend/backend isolation or any runtime adapter boundary.

Affected files:

- `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` — version declaration
- `src-tauri/tauri.conf.json` — version declaration and `bundle.targets`
- `.github/workflows/package.yml` — validation path, pre-release publication, release notes source
- `scripts/check-readme-parity.mjs`, `scripts/check-readme-parity.node-test.mjs` — version fact pattern
- `README.md`, `README.zh-CN.md`, `README.ja.md` — version fact, version badge, download routing
- `docs/release-signing.md` — preview release procedure

Downstream consumers: users who require an MSI or RPM package lose that format. Anyone consuming `SHA256SUMS`, the SPDX SBOM, or build provenance attestations is unaffected; those steps stay in place for preview releases.
