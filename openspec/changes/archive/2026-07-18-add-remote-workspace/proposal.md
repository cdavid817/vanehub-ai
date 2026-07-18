# add-remote-workspace

## Summary
Add session creation support for SSH remote workspaces alongside existing local project and Git worktree sessions.

## Motivation
Users need to start Agent sessions against a remote machine or container workspace without forcing the path through local filesystem inspection. The current session creation flow only accepts local paths, so remote targets cannot be represented consistently in desktop persistence, Web previews, or session search.

## Scope
- Add a remote workspace session input and session metadata contract.
- Persist remote workspace metadata in the desktop SQLite session store.
- Keep remote workspace behavior consistent in the Web/mock adapter.
- Add create-session UI controls for local versus remote workspace mode.
- Treat local file, document, Git, and shell workspace tabs as unavailable for remote workspace sessions until remote filesystem execution is implemented.

## Non-Goals
- SSH authentication, network connection testing, remote file browsing, remote Git status, or remote shell execution.
- Remote workspace synchronization or cloning.
