//! What a `.vhext` may contain, and every way it may not.

use super::{
    all_package_layout_rejections, check_manifest_against_layout, inspect_package_layout,
    ExtensionPackageLimits, PackageArchiveEntry, PackageLayoutRejection, PackageLayoutViolation,
    PathRejection, PortablePackagePath, DEFAULT_EXTENSION_PACKAGE_LIMITS, PACKAGE_DIRECTORIES,
    PACKAGE_MANIFEST_ENTRY, PACKAGE_ROOT_FILES,
};

const LIMITS: ExtensionPackageLimits = DEFAULT_EXTENSION_PACKAGE_LIMITS;

fn file(path: &str, expanded_bytes: u64) -> PackageArchiveEntry {
    PackageArchiveEntry {
        path: path.to_string(),
        is_directory: false,
        expanded_bytes,
        unix_mode: None,
    }
}

fn directory(path: &str) -> PackageArchiveEntry {
    PackageArchiveEntry {
        path: path.to_string(),
        is_directory: true,
        expanded_bytes: 0,
        unix_mode: None,
    }
}

fn manifest_entry() -> PackageArchiveEntry {
    file(PACKAGE_MANIFEST_ENTRY, 512)
}

fn inspect(entries: &[PackageArchiveEntry]) -> Result<(), PackageLayoutViolation> {
    inspect_package_layout(entries, 1_024, LIMITS).map(|_| ())
}

fn reason(entries: &[PackageArchiveEntry]) -> PackageLayoutRejection {
    inspect_package_layout(entries, 1_024, LIMITS)
        .expect_err("the package should be refused")
        .reason
}

#[test]
fn an_ordinary_package_is_admissible() {
    assert_eq!(
        inspect(&[
            manifest_entry(),
            directory("runtime/"),
            file("runtime/extension.wasm", 4_096),
            file("schemas/tool-input.json", 512),
            file("skills/reviewer/SKILL.md", 1_024),
            file("assets/icon.png", 2_048),
            file("README.md", 256),
            file("LICENSE", 1_024),
        ]),
        Ok(())
    );
}

#[test]
fn the_admissible_locations_are_exactly_what_the_allowlist_names() {
    // The allowlist is the rule; this pins that the rule and the constants are the same thing, so
    // adding a directory to one without the other is visible here rather than at install time.
    let entries = PACKAGE_DIRECTORIES
        .iter()
        .map(|directory| file(&format!("{directory}/thing"), 1))
        .chain(PACKAGE_ROOT_FILES.iter().map(|name| file(name, 1)))
        .chain([manifest_entry()])
        .collect::<Vec<_>>();

    let layout = inspect_package_layout(&entries, 1_024, LIMITS).expect("layout");
    assert_eq!(
        layout.files().len(),
        PACKAGE_DIRECTORIES.len() + PACKAGE_ROOT_FILES.len() + 1
    );
}

#[test]
fn a_package_without_a_manifest_is_refused() {
    assert_eq!(
        reason(&[file("runtime/extension.wasm", 4_096)]),
        PackageLayoutRejection::MissingManifest
    );
}

#[test]
fn entry_names_are_held_to_the_declared_path_rule_and_not_the_archive_floor() {
    // Each of these passes `crate::platform::archive`'s mechanical entry-name check and is refused
    // here. That difference is the whole reason this layer parses the name again.
    let cases = [
        ("runtime/CON", PathRejection::WindowsReservedName),
        ("runtime/entry.wasm ", PathRejection::TrailingDotOrSpace),
        ("runtime/nul\u{0}.wasm", PathRejection::NulByte),
        ("runtime/data.txt:$DATA", PathRejection::AlternateDataStream),
    ];
    for (name, expected) in cases {
        assert_eq!(
            reason(&[manifest_entry(), file(name, 1)]),
            PackageLayoutRejection::UnsafePath(expected),
            "{name}"
        );
    }

    // And traversal, which both layers refuse.
    assert_eq!(
        reason(&[manifest_entry(), file("../escape.wasm", 1)]),
        PackageLayoutRejection::UnsafePath(PathRejection::ParentDirectorySegment)
    );
}

#[test]
fn only_the_documented_top_level_locations_are_admissible() {
    for refused in [
        "bin/helper",
        "runtime2/entry.wasm",
        "vanehub-extension.yml",
        "setup.py",
        "docs/guide.md",
    ] {
        assert_eq!(
            reason(&[manifest_entry(), file(refused, 1)]),
            PackageLayoutRejection::UnexpectedLocation,
            "{refused}"
        );
    }
}

#[test]
fn two_entries_that_are_one_file_on_a_real_filesystem_are_refused() {
    let case = reason(&[
        manifest_entry(),
        file("schemas/Tool.json", 1),
        file("schemas/tool.json", 1),
    ]);
    assert_eq!(
        case,
        PackageLayoutRejection::CaseCollision {
            other: "schemas/Tool.json".to_string()
        }
    );

    // `é` as one code point and as `e` plus a combining accent: two names, one file on macOS.
    let unicode = reason(&[
        manifest_entry(),
        file("assets/caf\u{e9}.png", 1),
        file("assets/cafe\u{301}.png", 1),
    ]);
    assert_eq!(
        unicode,
        PackageLayoutRejection::UnicodeCollision {
            other: "assets/café.png".to_string()
        }
    );

    assert_eq!(
        reason(&[
            manifest_entry(),
            file("assets/a.png", 1),
            file("assets/a.png", 1)
        ]),
        PackageLayoutRejection::DuplicatePath
    );
}

#[test]
fn a_device_a_socket_or_a_fifo_is_refused_even_though_zip_has_no_name_for_one() {
    // ZIP has no entry kind for these; what it has is a Unix mode an archiver copied out of `stat`.
    for file_type in [0o020_000_u32, 0o060_000, 0o010_000, 0o140_000, 0o120_000] {
        let mut entry = file("assets/thing", 0);
        entry.unix_mode = Some(file_type | 0o644);
        assert_eq!(
            reason(&[manifest_entry(), entry]),
            PackageLayoutRejection::UnsupportedEntryKind,
            "{file_type:o}"
        );
    }

    // A regular file and a directory are fine, and so is an archive that recorded no mode at all,
    // which is ordinary for anything written on Windows.
    let mut regular = file("assets/thing", 0);
    regular.unix_mode = Some(0o100_644);
    assert_eq!(inspect(&[manifest_entry(), regular]), Ok(()));
}

#[test]
fn a_file_that_can_be_run_is_recorded_so_the_manifest_can_be_asked_about_it() {
    let layout = inspect_package_layout(
        &[
            manifest_entry(),
            file("runtime/helper.exe", 1),
            file("assets/icon.png", 1),
        ],
        1_024,
        LIMITS,
    )
    .expect("layout");

    assert_eq!(
        layout
            .executables()
            .iter()
            .map(PortablePackagePath::as_str)
            .collect::<Vec<_>>(),
        vec!["runtime/helper.exe"]
    );

    // The permission bits count too, for an archive written somewhere that records them.
    let mut marked = file("runtime/entry", 1);
    marked.unix_mode = Some(0o100_755);
    let by_mode =
        inspect_package_layout(&[manifest_entry(), marked], 1_024, LIMITS).expect("layout");
    assert_eq!(by_mode.executables().len(), 1);
}

#[test]
fn each_ceiling_is_accepted_exactly_and_refused_one_past_it() {
    let tiny = ExtensionPackageLimits {
        maximum_compressed_bytes: 1_000,
        maximum_expanded_bytes: 4_000,
        maximum_entries: 4,
        maximum_entry_bytes: 2_000,
        maximum_schema_bytes: 100,
        maximum_path_characters: LIMITS.maximum_path_characters,
        maximum_compression_ratio: 10,
    };
    let refuse = |entries: &[PackageArchiveEntry], compressed: u64| {
        inspect_package_layout(entries, compressed, tiny)
            .expect_err("refused")
            .reason
    };

    assert!(inspect_package_layout(&[manifest_entry()], 1_000, tiny).is_ok());
    assert_eq!(
        refuse(&[manifest_entry()], 1_001),
        PackageLayoutRejection::CompressedSize
    );

    let five = (0..4)
        .map(|index| file(&format!("assets/{index}.png"), 1))
        .chain([manifest_entry()])
        .collect::<Vec<_>>();
    assert_eq!(refuse(&five, 1_000), PackageLayoutRejection::EntryCount);

    assert_eq!(
        refuse(&[manifest_entry(), file("assets/big.png", 2_001)], 1_000),
        PackageLayoutRejection::EntryTooLarge
    );
    assert_eq!(
        refuse(
            &[
                manifest_entry(),
                file("assets/a.png", 2_000),
                file("assets/b.png", 2_000)
            ],
            1_000
        ),
        PackageLayoutRejection::ExpandedSize
    );
    assert_eq!(
        refuse(&[manifest_entry(), file("schemas/big.json", 101)], 1_000),
        PackageLayoutRejection::SchemaTooLarge
    );
}

#[test]
fn a_package_that_claims_to_expand_far_beyond_its_size_is_refused() {
    let tiny = ExtensionPackageLimits {
        maximum_compression_ratio: 10,
        ..DEFAULT_EXTENSION_PACKAGE_LIMITS
    };

    assert!(inspect_package_layout(&[file(PACKAGE_MANIFEST_ENTRY, 1_000)], 100, tiny).is_ok());
    assert_eq!(
        inspect_package_layout(&[file(PACKAGE_MANIFEST_ENTRY, 1_001)], 100, tiny)
            .expect_err("refused")
            .reason,
        PackageLayoutRejection::CompressionRatio
    );

    // An empty archive is not a bomb, and the ratio of nothing to anything is not a number worth
    // acting on.
    assert!(inspect_package_layout(&[file(PACKAGE_MANIFEST_ENTRY, 0)], 0, tiny).is_ok());
}

#[test]
fn the_running_total_saturates_into_a_rejection_rather_than_wrapping() {
    // The total is accumulated before the per-entry ceiling is consulted, because a directory
    // entry carries no per-entry budget and its declared bytes still have to count. So an absurd
    // single entry is reported against the total.
    assert_eq!(
        reason(&[manifest_entry(), file("assets/a.png", u64::MAX)]),
        PackageLayoutRejection::ExpandedSize
    );

    // With no ceiling at all the addition itself is what has to hold, and it does: two maximal
    // entries report a refusal rather than wrapping to a small number and being admitted.
    let unbounded = ExtensionPackageLimits {
        maximum_expanded_bytes: u64::MAX,
        maximum_entry_bytes: u64::MAX,
        maximum_compression_ratio: u64::MAX,
        ..DEFAULT_EXTENSION_PACKAGE_LIMITS
    };
    assert_eq!(
        inspect_package_layout(
            &[
                manifest_entry(),
                file("assets/a.png", u64::MAX),
                file("assets/b.png", u64::MAX),
            ],
            1_024,
            unbounded
        )
        .expect_err("refused")
        .reason,
        PackageLayoutRejection::ExpandedSize
    );
}

#[test]
fn every_rejection_has_a_distinct_stable_code() {
    let rejections = all_package_layout_rejections();
    let total = rejections.len();
    let mut codes: Vec<&str> = rejections
        .iter()
        .map(PackageLayoutRejection::code)
        .collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);
}

#[test]
fn the_manifest_pass_reports_declared_paths_the_package_does_not_contain() {
    let layout = inspect_package_layout(
        &[manifest_entry(), file("runtime/extension.wasm", 4_096)],
        1_024,
        LIMITS,
    )
    .expect("layout");
    let manifest =
        super::manifest_test_support::manifest_with_runtime_entry("runtime/missing.wasm");

    let violations = check_manifest_against_layout(&manifest, &layout);
    assert_eq!(
        violations,
        vec![PackageLayoutViolation {
            entry: "runtime/missing.wasm".to_string(),
            reason: PackageLayoutRejection::DeclaredPathMissing,
        }]
    );
}

#[test]
fn an_executable_the_manifest_did_not_declare_is_refused() {
    // A `.dll` beside a declared `.wasm` is not cargo: it is a second program nobody reviewed,
    // sitting inside a snapshot the runtime can reach.
    let layout = inspect_package_layout(
        &[
            manifest_entry(),
            file("runtime/extension.wasm", 4_096),
            file("runtime/helper.dll", 2_048),
        ],
        1_024,
        LIMITS,
    )
    .expect("layout");
    let manifest =
        super::manifest_test_support::manifest_with_runtime_entry("runtime/extension.wasm");

    let violations = check_manifest_against_layout(&manifest, &layout);
    assert_eq!(
        violations,
        vec![PackageLayoutViolation {
            entry: "runtime/helper.dll".to_string(),
            reason: PackageLayoutRejection::UndeclaredExecutable,
        }]
    );
}

#[test]
fn the_declared_runtime_entry_is_the_one_executable_a_package_may_carry() {
    let layout = inspect_package_layout(
        &[manifest_entry(), file("runtime/extension.wasm", 4_096)],
        1_024,
        LIMITS,
    )
    .expect("layout");
    let manifest =
        super::manifest_test_support::manifest_with_runtime_entry("runtime/extension.wasm");

    assert!(check_manifest_against_layout(&manifest, &layout).is_empty());
}
