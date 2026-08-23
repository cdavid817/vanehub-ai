//! What a seed may do to a binding a user already has.

use super::{
    all_hook_binding_errors, decide_seed, HookBinding, HookBindingError, HookGlobalId, HookScope,
    HookScopeKind, SeedOutcome,
};

fn binding(enabled: bool) -> HookBinding {
    HookBinding {
        hook: HookGlobalId::parse("vanehub.session-start").expect("hook"),
        scope: HookScope::global(),
        enabled,
        revision: 3,
        updated_at: "2026-08-01T00:00:00Z".to_string(),
    }
}

#[test]
fn a_seed_creates_a_binding_only_where_the_user_has_none() {
    assert_eq!(decide_seed(None), SeedOutcome::Seeded);
}

#[test]
fn a_seed_never_overwrites_a_binding_the_user_turned_off() {
    // The failure this exists to prevent: a built-in Hook is disabled, an upgrade re-seeds its
    // defaults, and the Hook starts running again. The user finds out when it runs.
    let disabled = binding(false);

    assert_eq!(decide_seed(Some(&disabled)), SeedOutcome::Preserved);
}

#[test]
fn a_seed_leaves_an_enabled_binding_alone_too() {
    // Not a special case for `false`. Rewriting an enabled binding would bump its revision and
    // hand a stale-revision failure to whoever was editing it at the time.
    assert_eq!(decide_seed(Some(&binding(true))), SeedOutcome::Preserved);
}

#[test]
fn every_seed_outcome_has_a_distinct_stable_code() {
    assert_ne!(SeedOutcome::Seeded.code(), SeedOutcome::Preserved.code());
}

#[test]
fn a_stale_revision_reports_both_numbers() {
    // "Someone else changed it" is not actionable; "you had 3, it is now 5" is.
    let error = HookBindingError::StaleRevision {
        expected: 3,
        actual: 5,
    };

    assert_eq!(error.code(), "hook_binding_stale_revision");
    let HookBindingError::StaleRevision { expected, actual } = error else {
        panic!("expected a stale revision");
    };
    assert_eq!((expected, actual), (3, 5));
}

#[test]
fn every_binding_failure_has_a_distinct_stable_code() {
    let errors = all_hook_binding_errors();
    let total = errors.len();

    let mut codes: Vec<&str> = errors.iter().map(HookBindingError::code).collect();
    codes.sort_unstable();
    codes.dedup();

    assert_eq!(codes.len(), total);
}

#[test]
fn a_binding_is_held_per_scope_so_one_scope_does_not_speak_for_another() {
    let global = binding(true);
    let project = HookBinding {
        scope: HookScope::scoped(HookScopeKind::Project, "d:/work/repo").expect("project"),
        enabled: false,
        ..binding(true)
    };

    assert_ne!(
        global.scope, project.scope,
        "distinct scopes are distinct rows; a project override must not overwrite the global one"
    );
}
