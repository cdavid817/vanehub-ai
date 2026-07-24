# Troubleshooting

## The CLI is unavailable

Run the provider command in a regular terminal. If the shell cannot find it, reinstall the CLI or correct the PATH visible to desktop applications, then restart VaneHub AI.

## The Agent asks me to sign in

Complete authentication in the provider CLI itself. VaneHub AI does not store your provider password.

## Browser preview says an operation succeeded

Check for the **Web/mock only** label. Browser preview uses deterministic simulations and does not prove that a native process, filesystem action, or SQLite write occurred.

## I cannot select Multi Agent

This is expected. Multi-Agent coordination has service/runtime support, but the creation UI remains a preview and is disabled.

## A screenshot differs locally

Documentation screenshots are authoritative in the pinned CI browser environment. Use `npm run docs:screenshots:update` only when intentionally reviewing an approved UI change.
