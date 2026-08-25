//! The session-run report, owned by the sessions context.
//!
//! Here rather than in observability or workspaces because the report is about a *session*: it
//! spans every context that did work under one. Any contributor could have hosted it, and each
//! would have made its own signal the centre of it.
//!
//! Nothing in production calls this yet — the adapters and the command arrive next, and until they
//! do the whole module is dead outside its own tests. `expect` rather than `allow` so that the
//! attribute becomes a clippy failure the moment it stops being true, and `cfg(not(test))` because
//! under `--all-targets` the tests below do use it, which would make a bare `expect` unfulfilled.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "wired to a command in 10.3/10.8; remove this attribute there"
    )
)]

mod models;
mod ports;
mod scope;
mod service;

#[cfg(test)]
mod tests;

// Nothing is re-exported yet. The re-export list is the module's public surface, and publishing one
// before a caller exists would fix the shape around what happens to be written rather than around
// what the adapters and the command turn out to need.
