# Provider SDK Security Rules

Provider registration and execution are fail-closed:

- Only reviewed providers in static composition are available. There is no directory scan, dynamic library load, package install/update, or external entrypoint.
- Manifests are data-only and strict. Executable values are basenames, not paths or shell fragments.
- Availability and version checks use bounded `--version`-style specifications. They never deliver prompts, create sessions, or start an interactive process.
- Output is read as bytes through separate bounded stdout/stderr buffers. Invalid UTF-8, malformed protocol data, and oversized records produce concise classified failures without copying raw payloads.
- Permission, resume, terminal, structured-output, usage and cancellation operations require declared capabilities before launch.
- Persistent diagnostics use the unified logging port. Secrets, prompts, environment values, sensitive arguments and unbounded raw output must not enter logs.

Enabling third-party providers requires a later specification covering provenance, signatures, permissions, Sandbox isolation, lifecycle, updates, disablement and quarantine.
