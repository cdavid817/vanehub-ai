//! How long a resolved approval decision should be remembered.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Scope {
    Once,
    Session,
    Project,
    Global,
}

// Whether a scope is remembered is no longer a question a caller asks and then acts on: it is
// answered by construction, in `RememberedScope::parse`, which cannot produce a binding from
// `Once`. The predicate that used to live here was a check every writer had to remember to
// perform — and one that a new persistence path could simply forget.
