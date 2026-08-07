# Troubleshooting

## The CLI is unavailable

Run the provider command in a regular terminal. If the shell cannot find it, reinstall the CLI or correct the PATH visible to desktop applications, then restart VaneHub AI.

## The Agent asks me to sign in

Complete authentication in the provider CLI itself. VaneHub AI does not store your provider password.

## Browser preview says an operation succeeded

Check for the **Web/mock only** label. Browser preview uses deterministic simulations and does not prove that a native process, filesystem action, or SQLite write occurred.

## A seat never gets the turn

Check the handle. Handles come from the expert role name, whitespace becomes `-`, and a repeated role name is suffixed. Mentions inside fenced code blocks are ignored on purpose. If a chain stopped because it hit the mention or depth bound, the reason is shown in the session.

## A screenshot differs locally

Documentation screenshots are authoritative in the pinned CI browser environment. Use `npm run docs:screenshots:update` only when intentionally reviewing an approved UI change.
