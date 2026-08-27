## Why

Java is the last language on the target list and the only one whose server does not fit the shape every other supported server shares. `jdtls` is not an executable on `PATH` speaking stdio with fixed arguments. It is an Eclipse archive launched through a JVM:

```
java -jar <install>/plugins/org.eclipse.equinox.launcher_<version>.jar \
     -configuration <install>/config_{win,mac,linux} \
     -data <per-workspace-data-directory>
```

Every clause there breaks an assumption the current model holds: the executable is `java`, the server's real identity lives in the arguments, the launcher jar name carries a version that must be resolved by glob, the configuration directory is platform-specific, and the data directory must differ per workspace.

## What Changes

This change makes Java **work**. It does not make Java **install itself** — that is the change after it, and the seam between them is one field.

Every other supported language is found on `PATH` or pointed at with an absolute executable path, and the user installs it themselves. Java is the same here: the user points VaneHub at an extracted `jdtls`, and everything downstream of that works. What differs is that "point at it" means a directory rather than a file, because for this launch shape the server is a directory.

- Add an interpreter launch shape to the language registry: a language may declare that its server runs through a host interpreter with a resolved argument template, rather than as a directly executable file.
- Make what a manual override *means* follow the launch shape. For an executable-shaped language it stays an absolute executable file, unchanged. For an interpreter-shaped one it is the server's install directory, validated by the presence of the artifact the template needs.
- Resolve the versioned launcher jar by bounded glob within that directory, failing closed with a distinct reason when zero or several match.
- Select the platform-specific configuration directory from the same install.
- Substitute a per-workspace data directory into the arguments, and remove it on trust revocation.
- Detect and report the JDK prerequisite separately from the server itself. A missing JDK is a distinct, actionable reason, not a generic server-start failure.
- Detect Java project roots from `pom.xml`, `build.gradle`, `build.gradle.kts`, and `settings.gradle`, under the same precedence rule the other multi-marker languages use.

Deferred to `manage-language-server-installation`, deliberately and not silently: the download, the tar.gz adapter and the dependency decision it carries, the install/upgrade/uninstall actions, and the settings entry points for them. That change supplies the install directory automatically where this one has the user supply it; nothing else about the launch path changes.

## Capabilities

### New Capabilities

None. Java is the fourth consumer of mechanisms established by the changes before it.

### Modified Capabilities

- `lsp-server-management`: the server-startup requirement admits an interpreter launch shape with bounded per-workspace argument substitution and glob-resolved launcher artifacts; the discovery requirement gains prerequisite detection reported separately from server availability, and defines what a manual override means for each launch shape; the lifecycle requirement covers per-workspace server data directories and their removal on trust revocation.
- `settings-center-ui`: the Java language card explains the JDK prerequisite and distinguishes "JDK missing" from "server directory not set" from "server directory does not contain a launcher".

## Impact

**Runtimes affected: desktop and Web.** The Web/mock adapter reports Java as a registered language with deterministic results, as it does for the other four.

Frontend/backend isolation is unchanged. The JVM launch, glob resolution, and per-workspace data directories are native concerns and stay in the Rust layer. The settings surface stays descriptor-driven — the card renders from what the backend reports, which is what `extend-lsp-language-registry` bought.

Affected code:

- `src-tauri/src/contexts/code_intelligence/domain/registry.rs` — the launch-shape field and the Java entry
- `src-tauri/src/contexts/code_intelligence/infrastructure/{server_discovery,project_root,server_test}.rs`
- `src-tauri/src/contexts/code_intelligence/api.rs` — argument resolution at launch, where the workspace is known
- `src/settings/pages/agents/` and the five locale bundles, for the prerequisite states

Known hazards this change must handle rather than discover late:

- Per-workspace data directories are state that outlives a session. Trust revocation must remove them, or a revoked workspace leaves an indexed copy of its source on disk.
- `jdtls` startup is slow and reports work-done progress for a long time. The existing distinction between protocol readiness and background indexing progress must hold here, or every early Java query looks like a timeout.
- The isolated server test must still complete `initialize` and `shutdown` within its deadline for a server much slower to start than the others. If it cannot, the deadline becomes a per-language declaration rather than a global constant, and that belongs in the registry — decided by measurement, not by guessing upward.
- A glob that matches several launcher jars means the install is not what it claims to be. Picking the newest would start a server the user cannot identify from the settings page.

Dependencies: `extend-lsp-language-registry` for the registry and `expand-lsp-read-only-methods` for the method set, both landed. `extract-managed-tool-installation` is **not** a dependency of this change — it is a dependency of the one after it.
