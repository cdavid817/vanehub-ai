# VaneHub AI 1.0.0

VaneHub AI 1.0.0 is the first stable desktop release of the unified workspace for Claude Code, Codex CLI, OpenCode, Gemini CLI, Antigravity CLI, and the built-in OnePiece API agent.

## Highlights

- Detect, install, configure, launch, and switch between supported coding agents from one workspace.
- Organize sessions, projects, Git worktrees, local terminals, and remote SSH workspaces.
- Manage MCP servers, SDKs, Skills, prompt hooks, extensions, scheduled tasks, notifications, and provider profiles.
- Inspect usage, task output, and redacted unified logs without bypassing the desktop service boundary.
- Use the browser-accessible Web/mock runtime for interface evaluation when native desktop capabilities are unavailable.

## Downloads

| Platform | Architecture | Assets |
| --- | --- | --- |
| Windows | x64 | Signed per-user `.exe` installer (NSIS) |
| macOS | Apple Silicon | Signed and notarized `aarch64` `.dmg` |
| macOS | Intel | Signed and notarized `x64` `.dmg` |
| Linux | x64 | `.deb` and AppImage |
| Linux | ARM64 | `.deb` and AppImage |

No Windows ARM64, `.msi`, or `.rpm` package is included in this release. Linux ARM64 packages are built on GitHub's `ubuntu-24.04-arm` hosted runner, whose label is currently in public preview.

## Verify your download

Stable publication is blocked unless the workflow verifies the Windows publisher and trusted timestamp and verifies macOS Developer ID signing, notarization, and stapled tickets. Linux packages do not use operating-system code signing; their evidence is integrity and provenance only.

Every downloadable package is covered by `SHA256SUMS`, an SPDX SBOM, and GitHub build-provenance and SBOM attestations. Download `SHA256SUMS` beside the package and run:

```bash
sha256sum --check --ignore-missing SHA256SUMS
```

Verify GitHub provenance with:

```bash
gh attestation verify <downloaded-file> --repo cdavid817/vanehub-ai
```

## Updates

Stable installations use signed updater artifacts and the stable update channel. Automatic checks are disabled by default and can be enabled from the application settings. Preview releases remain on a separate channel and are not offered to stable clients.

## Known limitations

- Native CLI detection, process launch, local storage, desktop integration, and installation require the Tauri desktop application; browser mode uses deterministic Web/mock behavior.
- Agent vendor authentication still occurs through each vendor's CLI or account flow; VaneHub AI does not broker subscription sign-in.
- Package availability is limited to the platform and architecture matrix above.

## Reporting problems

Open a [bug report](https://github.com/cdavid817/vanehub-ai/issues/new?template=bug.yml) with the release version, operating system, and a relevant redacted log excerpt. Use the [feature request](https://github.com/cdavid817/vanehub-ai/issues/new?template=feature.yml) for product ideas. Report vulnerabilities privately through a [GitHub security advisory](https://github.com/cdavid817/vanehub-ai/security/advisories/new), not a public issue.

---
