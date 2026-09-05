//! Startup-ordering assertions for the desktop builder.
//!
//! The single-instance guard's correctness lives in the order plugins are registered and in which
//! process claims the lock, neither of which a unit test can observe from a running application.
//! These assertions read the wiring itself.
//!
//! Prose is stripped before any scan. These files explain the rules they follow, so a scan that
//! read the explanation would stay green on a registration that had been deleted and left
//! described. Offsets are compared rather than lines, because the repository is checked out with
//! CRLF endings on Windows and line-anchored parsing is green on a freshly written file and red on
//! a clean checkout.

use super::runtime::guards_duplicate_launches;

const RUNTIME_SOURCE: &str = include_str!("runtime.rs");
const LIB_SOURCE: &str = include_str!("../lib.rs");

fn code_of(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//") && !trimmed.starts_with("/*") && !trimmed.starts_with('*')
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn offset_of(source: &str, needle: &str) -> usize {
    source
        .find(needle)
        .unwrap_or_else(|| panic!("`{needle}` is missing from the wiring under test"))
}

#[test]
fn the_desktop_builder_registers_a_single_instance_guard() {
    assert!(
        code_of(RUNTIME_SOURCE).contains(".plugin(tauri_plugin_single_instance::init"),
        "the desktop builder must register a single-instance guard, or a second launch starts a \
         second process against the same profile"
    );
}

#[test]
fn the_guard_is_registered_before_every_other_plugin() {
    // Derived from the wiring rather than compared against a list of plugin names, which nothing
    // would force a newly added plugin to join.
    let code = code_of(RUNTIME_SOURCE);
    let guard = offset_of(&code, ".plugin(tauri_plugin_single_instance::init");
    let first = code
        .match_indices(".plugin(")
        .map(|(index, _)| index)
        .min()
        .expect("the desktop builder registers at least one plugin");

    assert_eq!(
        guard, first,
        "the single-instance guard must be the first plugin registered, so a duplicate launch \
         short-circuits before it starts building a runtime it is about to abandon"
    );
}

#[test]
fn a_duplicate_launch_restores_the_running_main_window() {
    let code = code_of(RUNTIME_SOURCE);
    let restore = offset_of(&code, "fn restore_main_window_for_duplicate_launch");
    // Bounded to the function's own body, so a call belonging to some later function cannot stand
    // in for the sequence under test. `\nfn ` delimits under both LF and CRLF checkouts.
    let tail = &code[restore + "fn restore_main_window_for_duplicate_launch".len()..];
    let body = tail.find("\nfn ").map_or(tail, |end| &tail[..end]);

    for step in ["show()", "unminimize()", "set_focus()"] {
        assert!(
            body.contains(step),
            "restoring the running instance must call `{step}`: a duplicate launch can arrive \
             while the window is hidden in the tray, minimized, or merely behind another window, \
             and no single call covers all three"
        );
    }
}

#[test]
fn only_a_release_build_claims_the_instance_lock() {
    // Two halves, because neither is enough alone. The first pins the answer for the build this
    // test runs in; the second pins what that answer is derived from, so the exemption cannot
    // quietly go back to keying on a feature name -- which a typo would silently disable, taking
    // the guard out of production rather than out of the test client.
    assert_eq!(
        guards_duplicate_launches(),
        !cfg!(debug_assertions),
        "only a release build may claim the lock: `tauri dev` and the `tauri build --debug` test \
         client both share the installed application's bundle identifier"
    );

    let code = code_of(RUNTIME_SOURCE);
    let predicate = offset_of(&code, "const fn guards_duplicate_launches");
    let tail = &code[predicate..];
    let body = tail.find("\nfn ").map_or(tail, |end| &tail[..end]);

    assert!(
        body.contains("debug_assertions"),
        "the exemption must key on the build profile rather than on a feature flag"
    );
}

#[test]
fn helper_process_dispatch_still_precedes_desktop_bootstrap() {
    // Load-bearing for the guard: the MCP relay re-executes this same binary. It returns before
    // `bootstrap::run`, so it never reaches the builder and is never counted as a duplicate launch.
    // Reordering these would make the primary instance steal focus every time a relay starts.
    let code = code_of(LIB_SOURCE);

    assert!(
        offset_of(&code, "try_run_from_process_args") < offset_of(&code, "bootstrap::run()"),
        "helper-process dispatch must precede desktop bootstrap, or the runtime treats its own \
         relay children as duplicate launches"
    );
}
