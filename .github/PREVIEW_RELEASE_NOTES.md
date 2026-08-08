> **This is a preview build.** The packages below are **unsigned and un-notarized**. Your operating system will warn you before running them, and the steps to proceed are in [Installing](#installing). Expect rough edges, and expect to reinstall rather than update — no automatic updater is configured.

## What works

- **Delivered:** CLI management, single-Agent sessions, interactive Agent terminals, session organization, project/worktree and SSH workspace tools, settings, MCP/SDK/Skills/Prompt Hooks/extensions, IM connectors, scheduled tasks, notifications, usage reporting, and unified redacted logs.
- **Preview:** Multi-Agent coordination has native and Web/mock service contracts, but the create-session UI still disables Multi Agent mode.
- **Not available yet:** the Multi-Agent coordination UI, and Japanese application UI resources — Japanese currently covers the README only.
- **Not available in this build:** Claude Code permission-hook management. The hook is a separate binary that these packages do not yet ship, so enabling it in settings reports an error rather than writing a hook Claude Code could not run. Every other permission surface is unaffected, and your global Claude Code settings are left untouched.

## Downloads

| Platform | Asset | Notes |
| --- | --- | --- |
| Windows x64 | `.exe` installer (NSIS) | Installs per-user; no administrator rights required |
| macOS Apple Silicon | `aarch64` `.dmg` | |
| macOS Intel | `x64` `.dmg` | |
| Linux x64 | `.deb` and AppImage | AppImage runs without installation |

No `.msi` is published: the Windows Installer format cannot represent a pre-release version number. No `.rpm` is published: the RPM version field cannot contain the hyphen a pre-release version requires. Use the `.exe` installer and the AppImage respectively.

## Installing

### macOS

A `.dmg` downloaded through a browser is quarantined, and because this build is not notarized macOS reports that the application **"is damaged and can't be opened"**. The application is not damaged — that is the message macOS uses for un-notarized software.

Drag the app to `/Applications` as usual, then clear the quarantine attribute:

```bash
xattr -cr "/Applications/VaneHub AI.app"
```

Open the application normally afterward.

### Windows

SmartScreen shows **"Windows protected your PC"** because the installer carries no Authenticode signature. Choose **More info**, then **Run anyway**.

### Linux

Install the `.deb` with your package manager, or mark the AppImage executable and run it directly:

```bash
chmod +x VaneHub-AI-*.AppImage
./VaneHub-AI-*.AppImage
```

Binaries are built on the current Ubuntu runner image and link against its glibc, so older distributions may refuse to start them.

## Verifying your download

Every asset is listed in `SHA256SUMS`, published alongside them. Download it and check the file you retrieved:

```bash
sha256sum --check --ignore-missing SHA256SUMS
```

An SPDX SBOM and GitHub build-provenance attestations are also attached. You can verify provenance with:

```bash
gh attestation verify <downloaded-file> --repo cdavid817/vanehub-ai
```

Checksums, the SBOM, and attestations establish that these files came from this repository's build. **They do not replace operating-system code signing or Apple notarization**, and they do not remove the warnings described above.

## Reporting problems

Open a [bug report](https://github.com/cdavid817/vanehub-ai/issues/new?template=bug.yml) and include the version from this release, your operating system, and the relevant excerpt from the application logs. For a feature idea, use the [feature request](https://github.com/cdavid817/vanehub-ai/issues/new?template=feature.yml) template. Please report security issues privately through a [security advisory](https://github.com/cdavid817/vanehub-ai/security/advisories/new) rather than a public issue.

---
