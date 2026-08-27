## Context

Three changes built toward this one:

- `extract-managed-tool-installation` shipped the audited download path and a bounded zip extractor, both tested and both with no production caller. Every archive symbol carries `expect(dead_code)` under `cfg(not(test))`, so wiring a caller *forces* those attributes off.
- `add-lsp-java-jdtls` shipped the interpreter launch shape and made a manual override mean an install directory. It left one seam: where that directory comes from.
- `extend-lsp-language-registry` made the settings surface descriptor-driven, so an install button appears from a reported field rather than from a language name.

## Goals / Non-Goals

Goals:

- Installing `jdtls` is a button, and uninstalling it is another.
- A second installable language is a registry entry plus, if its format is new, one format adapter — never a second set of bounds.
- The user is told the bytes are not checksum-verified, before they click.

Non-Goals:

- Version selection, upgrade-in-place, or a version catalog. `cli-environment-management` has all of that for CLI tools and it is a large amount of machinery; a language server that installs and uninstalls is the whole ask here.
- Installing the JDK. That is a system runtime, and VaneHub does not manage those.
- Making the other five languages installable. They are one line with a package manager the user already has, and wrapping that would add a second way to do something that already works.

## Decisions

### 1. The bytes are unverified, and the UI says so

Eclipse publishes `jdt-language-server-latest.tar.gz`. There is no digest that stays valid across releases, so pinning one means the install breaks the next time Eclipse publishes — a checksum that fails for the expected reason teaches a user to ignore checksums.

So the artifact is declared `Unverified`: HTTPS, exact-host allowlist applied per redirect hop, a byte ceiling enforced while reading, a deadline, and cancellation — but no checksum.

**This is the posture every shipped CLI vendor installer already has.** All three of them (`claude.ai`, `opencode.ai`, `antigravity.google`) declare `Unverified` today, and those are downloads VaneHub *executes*. This one is extracted, and the extraction is bounded against traversal, size, entry count, and links.

What makes it acceptable is not that it is safe in the abstract — it is that the user asked for this specific server from this specific vendor, and the surface says plainly that the download is not checksum-verified. Hiding that behind an install button is the part that would be wrong.

Rejected: fetching Eclipse's published `.sha256` alongside the archive. It defends against a corrupted mirror but not against the host, it is a second request under the same allowlist, and it asserts a layout this change cannot verify. Recorded as an open question rather than guessed at.

### 2. One guard, two format walks

`ExtractionGuard` already owns containment, the byte ceiling, and the entry count. The `tar.gz` adapter is a loop that reads entries and calls `admit` then `write_entry`, exactly as the zip adapter does. Neither adapter can write an entry the guard did not admit, because neither has the destination path until `admit` returns it.

The guard also grows one rule this change needs: **an entry that is not a regular file or a directory is refused.** Tar carries symlinks and hard links; zip can too. A link is the one entry type whose containment cannot be decided when it is written — it resolves at use, and a link that points inside the destination today points outside it after something else moves. Refusing them is simpler than tracking that, and `jdtls` does not need them.

### 3. `tar` is the dependency, and that is a decision

`flate2` is already present for gzip. `tar` is not. It is pure Rust, widely used, and maintained under the `rust-lang` umbrella's orbit; CI's Dependency Review is what actually vets it.

Recorded here so it reads as a decision. A change that adds a dependency in passing is one where nobody chose it.

### 4. The managed directory is derived, and the override always wins

`<app data>/lsp/<language id>/install`. One per language, not one per version — this change has no version selection, so a second directory would be a directory nothing ever chooses between.

Discovery order: manual override, then managed install, then unavailable. An override always wins, and uninstall never touches what an override names — a user who pointed at their own `jdtls` must not lose it by clicking a button about VaneHub's copy.

### 5. Extraction lands somewhere else and then moves

`ExtractionGuard` extracts into a directory it owns and removes it on any failure. The install action renames that directory into place only after extraction returns. An interrupted install therefore leaves nothing, rather than leaving a directory that looks installed and fails at launch with a missing launcher.

A rename across filesystems can fail, so the temporary directory is created under the same parent as the destination rather than in the system temp.

**Reversed during implementation: the install copies rather than renames.** `ExtractionGuard`'s directory is owned by a `TempDir` that removes it when the handle drops, so renaming it away leaves the handle pointing at nothing and the removal fires against a path the install now depends on. The choice was to make the guard's ownership conditional or to copy out of it; conditional ownership costs the property the guard exists for — that every failure path cleans up without the caller remembering to. The install therefore copies the tree to the destination and lets the handle drop normally, removing a partially copied destination if the copy fails. The observable behaviour the requirement asks for is unchanged: an interrupted install leaves nothing that looks installed. What is lost is atomicity — a copy is not a single filesystem operation — which matters only if the process dies mid-copy, and that case is covered by the removal-on-failure and by reinstall replacing rather than merging.

## Risks / Trade-offs

- **Unverified bytes.** Stated above and stated in the UI. It is the existing posture, not a new one, but it is the thing to look at hardest in review.
- **`latest` means the version changes under the user.** The settings card reports the launcher file name, which carries the version, so what is installed is at least visible. Pinning would need a version catalog, which is explicitly out of scope.
- **A running server holds its files open.** Uninstall stops the language's processes first. On Windows, removing a directory a process still has open simply fails, so this is ordering rather than politeness.
- **Two format adapters is where duplication starts.** The guard is what prevents it; the spec now says every format goes through the same checks, so a third adapter that reimplements them fails the requirement rather than only looking untidy.

## Migration Plan

No data migration. A user who already pointed at their own `jdtls` is unaffected — their override still wins, and no managed directory exists until they ask for one.

## Open Questions

- Whether to fetch Eclipse's published checksum file and verify against it. It raises the floor from "no verification" to "verified against the same host that served the bytes", which is worth something against a corrupted mirror and nothing against a compromised host. Left out because it asserts a layout this change cannot verify; worth revisiting if a second vendor with the same publishing shape appears.
- Whether upgrade belongs here later or in a change of its own. It needs a version catalog and a way to say what is installed against what is available, which is most of what `cli-environment-management` does for CLI tools.
