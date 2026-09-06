# Security Policy

## Supported versions

Security fixes target the **latest published release line** (the most recent non-prerelease version on the [Releases page](https://github.com/cdavid817/vanehub-ai/releases)) and the `main` branch. Older release lines do not receive security fixes; upgrade to the latest release to stay covered. Integration branches such as `dev` are development snapshots, not supported release lines.

This policy is deliberately version-free so it cannot drift from the release history; the automated documentation checks verify that no stale version number reappears here.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability. Use [GitHub private vulnerability reporting](https://github.com/cdavid817/vanehub-ai/security/advisories/new) so maintainers can investigate without exposing users.

Include the affected version or commit SHA, reproduction steps, likely impact, and any suggested mitigation. Remove credentials, tokens, personal data, and unredacted local logs from the report.

The project aims to acknowledge reports within seven days; as a community-maintained project this is a target, not a guaranteed SLA. Validation, remediation, disclosure timing, and release notes are coordinated through the private advisory. Please avoid public disclosure until a fix or agreed mitigation is available.

## Not a vulnerability?

- Usage questions and reproducible defects go through the public [issue forms](https://github.com/cdavid817/vanehub-ai/issues) — see [SUPPORT.md](SUPPORT.md).
- Conduct concerns follow the reporting path in the [Code of Conduct](CODE_OF_CONDUCT.md), not the security advisory channel.
