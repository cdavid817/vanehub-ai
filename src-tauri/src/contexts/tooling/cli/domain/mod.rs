//! The source-aware CLI environment model.
//!
//! One module per concern, and nothing but module declarations here. The flat `CliToolStatus`
//! model this replaced lived in this file: a `ToolDefinition` table, five status enums, a
//! `LifecycleEligibility` that mixed package managers with download transports, and the derivation
//! functions for all of it, in one 700-line blob with its own tests underneath. Splitting it is
//! what makes the dependency rules checkable -- a file that holds everything has no boundaries to
//! point at.
//!
//! Consumers name the module (`domain::ids::CliToolId`) rather than a re-export, so adding a type
//! does not also mean editing a list here.
//!
//! Several modules carry vocabulary the domain tests pin but no production path calls yet -- a
//! checksum algorithm no vendor definition declares, bulk progress the UI reads per item instead.
//! `cfg_attr(not(test), expect(dead_code))` is how that is marked: `expect` rather than `allow`,
//! so an item that stops being dead fails the build and gets its attribute removed, and
//! `not(test)` so the attribute doubles as a coverage gate -- an item no test touches fails the
//! test build rather than sitting here unnoticed.

/// Marks a module that still carries vocabulary only its own tests reach.
///
/// Written out per module rather than applied to all of them, so a module that has no unused item
/// left does not silently keep the attribute: `expect` fails the build when it has nothing to
/// suppress, which is what makes the list shrink as callers land.
macro_rules! test_pinned_module {
    ($name:ident) => {
        #[cfg_attr(
            not(test),
            expect(
                dead_code,
                reason = "domain vocabulary pinned by this context's tests; see the module docs"
            )
        )]
        pub(crate) mod $name;
    };
}

test_pinned_module!(action);
test_pinned_module!(bulk);
test_pinned_module!(definition);
test_pinned_module!(ids);
test_pinned_module!(installation);
test_pinned_module!(phase);
test_pinned_module!(plan);
test_pinned_module!(probe);
test_pinned_module!(probe_interpretation);
test_pinned_module!(snapshot);
test_pinned_module!(source);
test_pinned_module!(trust);

pub(crate) mod catalog;
pub(crate) mod operation_record;
pub(crate) mod registry;
pub(crate) mod status;
pub(crate) mod version;

pub(crate) use version::compare_versions;
