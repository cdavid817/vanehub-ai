## Context

Four languages are registered and all four share one shape: an executable on `PATH` (or an absolute override), fixed arguments, stdio. The registry entry says `executables` and `default_startup_arguments` and that is the whole launch.

`jdtls` is a directory. `java -jar <install>/plugins/org.eclipse.equinox.launcher_<version>.jar -configuration <install>/config_<platform> -data <per-workspace>`. The executable is the JVM, the server's identity is in the arguments, one argument's file name carries a version, another is platform-specific, and a third has to differ per workspace.

What the three changes before this one bought:

- The registry is one table, so a fifth language is data plus whatever code its shape genuinely needs.
- The settings surface is descriptor-driven, so a language card renders from what the backend reports.
- `managed-tool-installation` owns verified download and bounded extraction — used by the change *after* this one, not by this one.

## Goals / Non-Goals

Goals:

- Java code intelligence works for a user who has `jdtls` extracted somewhere and a JDK installed.
- The launch-shape extension is general: a second interpreter-launched server is a registry entry, not a second special case.
- The three ways Java can be unusable — no JDK, no install directory, a directory without a launcher — are three distinct reported reasons, because they need three different actions from the user.

Non-Goals:

- Downloading or installing `jdtls`. That is `manage-language-server-installation`, and the seam is a single field: where the install directory comes from.
- A `tar.gz` adapter, and the dependency decision it carries. Same reason.
- Making the argument template user-configurable.

## Decisions

### 1. The launch shape is a registry field, and it decides what the other fields mean

```rust
enum LaunchShape {
    Executable,
    Interpreter(&'static InterpreterLaunch),
}
```

`Executable` is what the four existing entries carry and nothing about them changes. Under `Interpreter`, `executables` names the *interpreter* candidates rather than the server, and the server lives in the template.

Rejected: a separate `InterpreterDefinition` table keyed by language id. Two tables means "which one is Java in" is a question, and the registry exists so that it is not.

### 2. The override's meaning follows the shape, and the frontend learns that from a descriptor

For `Executable` an override is an absolute executable file, unchanged. For `Interpreter` it is the install directory.

The settings card must not branch on `language === "java"`. It branches on a descriptor field the backend reports, the same way it already branches on `supportedOnHost`. A second interpreter-shaped language then needs no frontend change, which is the property `extend-lsp-language-registry` was built to have and would be quietly given up by one identity check.

### 3. Discovery reports which of four things is wrong, not that it failed

`PrerequisiteMissing`, `InstallDirectoryNotSet`, `OverrideMissing`, `LauncherNotFound`, `AmbiguousInstall`. Each needs a different action: install a JDK, point at a directory, fix the path, check the extraction, clean up a duplicate.

Ordered so the first missing thing is the one reported: a user with no JDK and no directory is told about the JDK first, because that is the one they hit first anyway.

### 4. Several launchers is a refusal, not a choice

Picking the newest would start a server whose version the settings page does not name. An install directory holding two `org.eclipse.equinox.launcher_*.jar` files is not the install the page describes, and saying so is more useful than guessing.

Matching is prefix-plus-suffix within one declared directory, not a glob library and not a recursive walk. The pattern is `org.eclipse.equinox.launcher_` … `.jar` inside `plugins/`, and nothing about that needs a dependency.

### 5. The data directory is derived, not stored

`<app data>/lsp/<language id>/<hash of canonical workspace root>`. Derived so there is no table to keep in sync with trust, and hashed so a workspace path never lands in a directory name.

Removed on trust revocation, in the same place the process is stopped. Not on idle shutdown: the directory is `jdtls`'s index, and throwing it away every time the server idles would make the next start pay for a full re-index. Trust revocation is the point at which the user has said they no longer want this workspace served.

### 6. User startup arguments append, they do not replace

Everywhere else, configured arguments replace the registry default — that rule exists so clearing the field means something. Under `Interpreter` the template is not the default, it is the launch, so configured arguments append after it. A template a user can replace is one they can replace with something that does not start a server.

## Risks / Trade-offs

- **The isolated server test has a fixed deadline and `jdtls` is slow to start.** If it cannot complete `initialize` and `shutdown` in time, the deadline becomes a per-language registry declaration. That is decided by measuring, not by raising it pre-emptively — a deadline raised on a guess stops being a deadline.
- **`jdtls` reports work-done progress for a long time after `initialize`.** The existing separation of protocol readiness from indexing progress has to hold, or every early Java query reads as a timeout. It is already implemented; this is a thing to verify rather than build.
- **Without managed installation, Java is usable only by someone who already has `jdtls`.** Accepted deliberately: it is the same position every other language is in, and the next change removes the step. The alternative — one change carrying a new launch shape *and* a new archive format *and* a new dependency *and* install orchestration — is one nobody can review, and a failure in it is unattributable.
- **A per-workspace directory outlives the session.** If revocation cleanup is wrong, a revoked workspace leaves an index of its source on disk. That is the one failure here with a privacy shape, so it gets a test that asserts the directory is gone rather than that the call was made.

## Migration Plan

No data migration. Java arrives disabled like every other language, and `lsp_language_configurations` already accepts any registered id — migration 86 removed the `CHECK` that would have needed changing.

## Open Questions

- Whether the JDK version matters enough to detect. `jdtls` needs 17 or newer, and running it under an older JVM fails with a message from Eclipse rather than from VaneHub. Detecting the version means running `java -version` and parsing it, which is a real cost for a check that may not earn it. Left out here; if the failure turns out to be common, the reason code already exists to carry it.
