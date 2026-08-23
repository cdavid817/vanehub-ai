//! The authority chain against a real database.
//!
//! Every test here builds the chain the hard way — snapshot, installation pointer, runtime
//! generation, active pointer — because the defect this reader exists to prevent is precisely a
//! consumer that skipped a link and answered from "whatever was recorded most recently".

use super::{
    record_snapshot_detail, RecordedContribution, SqliteActiveContributionReader,
    SqliteRuntimeGenerationRepository, SqliteSnapshotPointerRepository,
};
use crate::contexts::tooling::extension_platform::application::{
    ActiveContributionReader, RuntimeGenerationRepository, SnapshotPointerRepository,
};
use crate::contexts::tooling::extension_platform::domain::{
    ActiveContribution, ActiveContributionError, ExtensionId, InstallationId, ManifestDigest,
    PackageHash, RuntimeGenerationId, RuntimeGenerationRecord, SnapshotId, SnapshotRecord,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use semver::Version;
use std::sync::Arc;

const V1_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const V2_DIGEST: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const HOOK: &str = "ext::acme.git-guardian::pre-commit";
const AT: &str = "2026-08-22T00:00:00Z";

struct Fixture {
    _directory: TempDirectory,
    database: Arc<NativeDatabase>,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    let database = Arc::new(
        NativeDatabase::new(directory.path().to_path_buf()).expect("database should open"),
    );
    Fixture {
        _directory: directory,
        database,
    }
}

/// Publishes one version of one extension and records what its snapshot contributes.
fn publish(
    fixture: &Fixture,
    installation: &str,
    extension: &str,
    snapshot: &str,
    version: &str,
    expected_revision: i64,
    contributes: Option<&str>,
) {
    SqliteSnapshotPointerRepository::new(
        fixture.database.clone(),
        InstallationId::parse(installation).expect("installation"),
    )
    .point_at(
        &SnapshotRecord {
            snapshot: SnapshotId::parse(snapshot).expect("snapshot"),
            extension: ExtensionId::parse(extension).expect("extension"),
            version: Version::parse(version).expect("version"),
            package_hash: PackageHash::parse(V1_DIGEST).expect("hash"),
            manifest_digest: ManifestDigest::parse(V1_DIGEST).expect("digest"),
            created_at: AT.to_string(),
        },
        expected_revision,
    )
    .expect("publish");

    if let Some(digest) = contributes {
        record_snapshot_detail(
            &fixture.database,
            &SnapshotId::parse(snapshot).expect("snapshot"),
            &[],
            &[RecordedContribution {
                global_id: HOOK.to_string(),
                kind: "hook".to_string(),
                local_id: "pre-commit".to_string(),
                declared_digest: Some(digest.to_string()),
            }],
        )
        .expect("detail");
    }
}

/// Makes one already-published snapshot the running one.
fn activate(fixture: &Fixture, installation: &str, generation: &str, snapshot: &str, at: i64) {
    let repository = SqliteRuntimeGenerationRepository::new(fixture.database.clone());
    let installation = InstallationId::parse(installation).expect("installation");
    repository
        .record(&RuntimeGenerationRecord {
            generation: RuntimeGenerationId::parse(generation).expect("generation"),
            installation: installation.clone(),
            snapshot: SnapshotId::parse(snapshot).expect("snapshot"),
            started_at: AT.to_string(),
        })
        .expect("record generation");
    repository
        .activate(
            &installation,
            &RuntimeGenerationId::parse(generation).expect("generation"),
            at,
            AT,
        )
        .expect("activate");
}

fn reader(fixture: &Fixture) -> SqliteActiveContributionReader {
    SqliteActiveContributionReader::new(fixture.database.clone())
}

#[test]
fn every_answer_has_a_distinct_stable_code() {
    // The three "no" answers are different answers. A caller that logged them under one code could
    // not tell "installed but not running" from "uninstalled", which is the distinction an
    // operator looking at a Hook that will not fire actually needs.
    let answers = [
        ActiveContribution::Running {
            snapshot_id: String::new(),
            declared_digest: None,
        },
        ActiveContribution::NoActiveGeneration,
        ActiveContribution::NotInstalled,
    ];
    let mut codes: Vec<&str> = answers.iter().map(ActiveContribution::code).collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total);
}

#[test]
fn nothing_installed_contributing_the_id_reports_not_installed() {
    let fixture = fixture("active-not-installed");

    assert_eq!(
        reader(&fixture).active(HOOK).expect("read"),
        ActiveContribution::NotInstalled
    );
}

#[test]
fn an_installation_with_no_active_generation_is_installed_but_not_running() {
    // The state right after publishing and before activating. A reader that collapsed this into
    // "not installed" would make an operator think the extension had vanished.
    let fixture = fixture("active-no-generation");
    publish(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snap-v1",
        "1.0.0",
        0,
        Some(V1_DIGEST),
    );

    assert_eq!(
        reader(&fixture).active(HOOK).expect("read"),
        ActiveContribution::NoActiveGeneration
    );
}

#[test]
fn the_running_snapshot_is_the_one_the_active_pointer_names() {
    let fixture = fixture("active-running");
    publish(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snap-v1",
        "1.0.0",
        0,
        Some(V1_DIGEST),
    );
    activate(&fixture, "install-a", "generation-1", "snap-v1", 0);

    assert_eq!(
        reader(&fixture).active(HOOK).expect("read"),
        ActiveContribution::Running {
            snapshot_id: "snap-v1".to_string(),
            declared_digest: Some(V1_DIGEST.to_string()),
        }
    );
}

#[test]
fn a_newer_snapshot_recorded_but_not_activated_does_not_become_the_answer() {
    // The defect in one test. v2 is published and its contributions recorded, so it is the most
    // recently recorded everything -- and the pointer still says v1.
    let fixture = fixture("active-recorded-not-activated");
    publish(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snap-v1",
        "1.0.0",
        0,
        Some(V1_DIGEST),
    );
    activate(&fixture, "install-a", "generation-1", "snap-v1", 0);
    publish(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snap-v2",
        "2.0.0",
        1,
        Some(V2_DIGEST),
    );

    assert_eq!(
        reader(&fixture).active(HOOK).expect("read"),
        ActiveContribution::Running {
            snapshot_id: "snap-v1".to_string(),
            declared_digest: Some(V1_DIGEST.to_string()),
        },
        "the recorded-but-unactivated v2 must not win"
    );
}

#[test]
fn activating_and_then_rolling_back_moves_the_answer_both_ways() {
    let fixture = fixture("active-rollback");
    publish(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snap-v1",
        "1.0.0",
        0,
        Some(V1_DIGEST),
    );
    publish(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snap-v2",
        "2.0.0",
        1,
        Some(V2_DIGEST),
    );
    activate(&fixture, "install-a", "generation-1", "snap-v1", 0);
    activate(&fixture, "install-a", "generation-2", "snap-v2", 1);

    assert_eq!(
        reader(&fixture).active(HOOK).expect("read"),
        ActiveContribution::Running {
            snapshot_id: "snap-v2".to_string(),
            declared_digest: Some(V2_DIGEST.to_string()),
        }
    );

    // Rolling back activates a fresh generation over the older snapshot. v2 remains the most
    // recently *recorded* revision throughout, which is why recording order cannot decide this.
    activate(&fixture, "install-a", "generation-3", "snap-v1", 2);

    assert_eq!(
        reader(&fixture).active(HOOK).expect("read"),
        ActiveContribution::Running {
            snapshot_id: "snap-v1".to_string(),
            declared_digest: Some(V1_DIGEST.to_string()),
        }
    );
}

#[test]
fn a_running_snapshot_that_dropped_the_contribution_declares_nothing_for_it() {
    // v2 no longer ships the Hook. The extension is still identified through v1's contribution
    // row -- that lookup only finds the owner -- but the running snapshot has nothing to declare.
    let fixture = fixture("active-dropped");
    publish(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snap-v1",
        "1.0.0",
        0,
        Some(V1_DIGEST),
    );
    publish(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snap-v2",
        "2.0.0",
        1,
        None,
    );
    activate(&fixture, "install-a", "generation-2", "snap-v2", 0);

    assert_eq!(
        reader(&fixture).active(HOOK).expect("read"),
        ActiveContribution::Running {
            snapshot_id: "snap-v2".to_string(),
            declared_digest: None,
        }
    );
}

#[test]
fn two_installations_claiming_one_contribution_id_is_refused_rather_than_resolved() {
    // Prevented upstream by the id grammar and by admission; not prevented by the database, whose
    // key is `(snapshot_id, global_id)`. Picking one of two owners would silently dispatch an
    // extension the operator did not install for that id.
    let fixture = fixture("active-ambiguous");
    publish(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snap-a",
        "1.0.0",
        0,
        Some(V1_DIGEST),
    );
    publish(
        &fixture,
        "install-b",
        "other.impostor",
        "snap-b",
        "1.0.0",
        0,
        Some(V2_DIGEST),
    );

    let error = reader(&fixture).active(HOOK).expect_err("ambiguous");

    assert_eq!(error, ActiveContributionError::AmbiguousOwner);
    assert_eq!(error.code(), "ambiguous_contribution_owner");
}

#[test]
fn the_three_reads_share_one_snapshot_across_a_concurrent_activation() {
    // Under WAL each bare statement takes its own snapshot, so without an explicit read
    // transaction this reader could return the owner from before an activation and the generation
    // from after it -- a state that never existed, assembled from two that did.
    //
    // The reader pauses once its snapshot is established; the writer then activates v2 on a second
    // connection and commits. Whatever the reader returns must be one whole generation.
    let fixture = fixture("active-snapshot-isolation");
    publish(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snap-v1",
        "1.0.0",
        0,
        Some(V1_DIGEST),
    );
    publish(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snap-v2",
        "2.0.0",
        1,
        Some(V2_DIGEST),
    );
    activate(&fixture, "install-a", "generation-1", "snap-v1", 0);

    let (reader_paused, paused) = std::sync::mpsc::channel();
    let (writer_done, activated) = std::sync::mpsc::channel();
    let reading = SqliteActiveContributionReader::new(fixture.database.clone());
    let reading_thread = std::thread::spawn(move || {
        reading.active_pausing_after_owner_lookup(HOOK, &|| {
            reader_paused.send(()).expect("signal");
            activated.recv().expect("wait for the writer");
        })
    });

    paused.recv().expect("the reader reached its pause");
    // A second, independent connection: the whole point is that this commits while the reader's
    // snapshot is open. Revision 1, because `generation-1` already moved the pointer.
    activate(&fixture, "install-a", "generation-2", "snap-v2", 1);
    writer_done.send(()).expect("release the reader");

    let answer = reading_thread.join().expect("thread").expect("read");

    assert!(
        matches!(
            &answer,
            ActiveContribution::Running { snapshot_id, declared_digest }
                if (snapshot_id == "snap-v1" && declared_digest.as_deref() == Some(V1_DIGEST))
                    || (snapshot_id == "snap-v2" && declared_digest.as_deref() == Some(V2_DIGEST))
        ),
        "the answer must be one whole generation, never a mixture: {answer:?}"
    );
    assert_eq!(
        answer,
        ActiveContribution::Running {
            snapshot_id: "snap-v1".to_string(),
            declared_digest: Some(V1_DIGEST.to_string()),
        },
        "and it is the generation that was live when the snapshot was taken"
    );

    // The writer's commit is visible to a read that starts after it, so the test proved isolation
    // rather than that the write never landed.
    assert_eq!(
        reader(&fixture).active(HOOK).expect("read"),
        ActiveContribution::Running {
            snapshot_id: "snap-v2".to_string(),
            declared_digest: Some(V2_DIGEST.to_string()),
        }
    );
}

#[test]
fn a_contribution_row_written_before_digests_existed_declares_nothing() {
    // The repaired-database case. A NULL digest is not an empty digest that matches nothing; it is
    // an absent declaration, and the conservative reading is "nothing to dispatch".
    let fixture = fixture("active-null-digest");
    publish(
        &fixture,
        "install-a",
        "acme.git-guardian",
        "snap-v1",
        "1.0.0",
        0,
        None,
    );
    record_snapshot_detail(
        &fixture.database,
        &SnapshotId::parse("snap-v1").expect("snapshot"),
        &[],
        &[RecordedContribution {
            global_id: HOOK.to_string(),
            kind: "hook".to_string(),
            local_id: "pre-commit".to_string(),
            declared_digest: None,
        }],
    )
    .expect("detail");
    activate(&fixture, "install-a", "generation-1", "snap-v1", 0);

    assert_eq!(
        reader(&fixture).active(HOOK).expect("read"),
        ActiveContribution::Running {
            snapshot_id: "snap-v1".to_string(),
            declared_digest: None,
        }
    );
}
