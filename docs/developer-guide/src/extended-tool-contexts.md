# Extended tool contexts

Beyond OnePiece's fixed native tools (shell, file, remember), four contexts each provide one category of **high-risk capability**: running code, driving a browser, reaching the public internet, storing artifacts.

| Context | Capability | Gate | Default |
| --- | --- | --- | --- |
| `code_execution` | Sandboxed code runtime | `VANEHUB_ONEPIECE_CODE_EXECUTION_ENABLED` | Disabled |
| `browser_automation` | Managed Playwright browser | `VANEHUB_ONEPIECE_BROWSER_ENABLED` | Disabled |
| `web_research` | Search and guarded fetching | `VANEHUB_ONEPIECE_WEB_ENABLED` | Disabled |
| `artifacts` | Content-addressed artifact storage | `VANEHUB_ONEPIECE_ARTIFACT_*` | Read enabled, publish/download disabled |

**Only OnePiece can discover or invoke these tools.** A custom API Agent or a CLI-wrapping Agent cannot acquire them by copying a display name, provider metadata, or capability label — desktop-side policy is authoritative, and React visibility is advisory only. The gate list and rollback triggers are in [OnePiece built-in tools](onepiece-builtin-tools.md).

## code_execution: seven capabilities, none optional

The sandbox is not "isolated as much as feasible" — `SandboxBackendCapabilities::ready()` requires **all seven** to hold before it's considered ready:

| Capability | Meaning |
| --- | --- |
| `restricted_identity` | Runs under a restricted identity |
| `job_cpu_limit` | CPU quota |
| `job_memory_limit` | Memory quota |
| `job_process_limit` | Process-count quota |
| `kill_process_tree` | Can kill the whole process tree |
| `acl_confinement` | ACL restricts file access |
| `network_denied` | No network |

```rust,ignore
pub(crate) const fn ready(self) -> bool {
    self.restricted_identity && self.job_cpu_limit && self.job_memory_limit
        && self.job_process_limit && self.kill_process_tree
        && self.acl_confinement && self.network_denied
}
```

**This all-of-them conjunction is deliberate.** Missing any single item leaves a gap in isolation — "CPU limited but network still open" sits at the same security tier as "no isolation at all." When a capability is missing, the backend reports `IsolationUnavailable`; the capability **does not degrade, it becomes unavailable**.

### Execution status separates "failed" from "went out of bounds"

`CodeExecutionStatus` has seven states:

| State | Meaning |
| --- | --- |
| `Succeeded` / `Failed` / `Cancelled` | Ordinary outcomes |
| `TimedOut` | Timed out |
| `LimitExceeded` | Hit a quota; `limit_reason` says which one |
| **`SandboxViolation`** | **The code attempted to break isolation** |
| **`CleanupFailed`** | **It ran to completion but cleanup didn't finish cleanly** |

The last two must stay separate from `Failed`: `Failed` means the code itself didn't work out, `SandboxViolation` is a security event, and `CleanupFailed` means something may have been left behind on the host — the three call for entirely different handling.

`stdout_truncated` / `stderr_truncated` in the result are explicit booleans — **the reader is never left to guess truncation from length**. The source-code cap is `MAX_SOURCE_BYTES = 128 KB`.

## web_research: URL admission is fail-closed

Before fetching, `GuardedUrlPolicy::resolve_public` resolves the URL to a concrete address first and judges from there. Eight rejection reasons:

| `GuardedUrlPolicyError` | What it blocks |
| --- | --- |
| `InvalidUrl` | The URL itself is malformed |
| `DisallowedScheme` | Not http/https |
| `CredentialsDisallowed` | The URL has an embedded username/password |
| `HostRequired` | No hostname |
| `PortDisallowed` | The port isn't allowed |
| `ResolutionFailed` | DNS resolution failed |
| `AddressDisallowed` | Resolves to a private, loopback, metadata, or documentation address |
| **`DnsRebinding`** | **Multiple DNS answers for the same host mix in a private address** |

**`DnsRebinding` is the easiest one to overlook.** An attacker can make one domain resolve to both a public address and a private one; a check that only inspects the public address would pass admission while the actual connection lands on the private one. There is a test in the repository named `private_metadata_documentation_and_mixed_dns_answers_fail_closed` — **mixed answers are rejected outright**, rather than betting on which address the connection ends up hitting.

**Admission happens after resolution, not before**: looking at the string alone can't tell you where `internal.example.com` actually points.

## browser_automation: the sidecar and handoff

The browser runs in an independent sidecar process; the context owns the protocol, session and action policy, operation lifecycle, and **artifact handoff** — screenshots, PDFs, and the like the browser produces aren't kept by it, but handed off to `artifacts`.

The rollback triggers are spelled out specifically: an orphaned sidecar, a policy bypass, profile leakage, or a handoff attribution failure. **"Orphaned sidecar" gets its own line** because a browser process outliving its host is the single most typical failure mode for this class of integration.

## artifacts: content addressing

`artifacts` owns content-addressed blobs: media-type and size validation, deduplication, and store capacity policy.

Content addressing means **the same content is stored only once** — repeated executions that produce identical results don't take up space again, and `content_hash` turns "has this artifact been modified" into a verifiable question.

`CodeOutputArtifact` is its interface with `code_execution`: `artifact_id`, `content_hash`, `relative_name`, `size_bytes`, `media_type`. Files produced by execution never expose the host path directly, only a logical identifier.

Three gates separate read, publish, and download:

- **Read** is enabled by default — listing, metadata, bounded reads, and review.
- **Publish** (`ARTIFACT_PUBLISH`) requires one-time confirmation and hash binding.
- **Download** (`ARTIFACT_DOWNLOAD`) requires hash verification, its own save path, size limits, and active-content handling.

**Download gets its own gate because it crosses the application boundary**: writing content to a path the user chooses themselves carries a completely different risk than reading a blob inside the application.

## The shared design stance

The four contexts differ mechanically, but they converge on the same stance:

- **Disabled by default**, and every gate is independent — rolling back one domain doesn't affect the others.
- **Partial isolation counts as no isolation** (the sandbox's all-of-seven conjunction, mixed DNS answers rejected outright).
- **Going out of bounds and failing are reported separately** (`SandboxViolation` vs. `Failed`, `CleanupFailed` on its own line).
- **The host path is never exposed to the model** — a logical identifier is used instead, always.

## Relationship to other contexts

- How tools enter OnePiece's catalog and get dispatched is in [Tool registry and execution](tool-registry.md) and [OnePiece native Agent](onepiece-native-agent.md).
- Isolated execution delegated to an external CLI is a different path; see [CLI delegation and the ChangeSet pipeline](cli-delegation.md).
- Gates, dependencies, and promotion/rollback criteria are in [OnePiece built-in tools](onepiece-builtin-tools.md).

## Where the design lives

This chapter orients contributors; the authoritative requirements live in the corresponding main specs under `openspec/specs`.
