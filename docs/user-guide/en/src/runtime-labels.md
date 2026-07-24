# Runtime and feature labels

| Label | Meaning |
| --- | --- |
| Delivered | Implemented user-visible path with verification evidence |
| Preview | Supporting contract exists, but the normal workflow is incomplete |
| Web/mock only | Deterministic browser simulation without native side effects |
| Desktop only | Tauri runtime with local filesystem, CLI, SQLite, or OS integration |
| Planned | No supported workflow yet |

When a page looks functional in Web preview, check its runtime label before assuming it changed the local machine. Simulated operations remain valuable for UI evaluation but are not evidence of native execution.
