// Included through `#[path]` from managed_server.rs.
//
// No test reaches the network. The retriever is a port precisely so the install path can be driven
// without one -- a test that fetched download.eclipse.org would fail on an air-gapped runner and
// pass for the wrong reason everywhere else.
use super::*;

use crate::contexts::code_intelligence::domain::registry;
use crate::contexts::tooling::api::RetrievedArtifact;

/// Serves a prepared archive instead of fetching one, and records what it was asked for.
struct FixtureRetriever {
    archive: Vec<u8>,
    requested: std::sync::Mutex<Vec<String>>,
}

impl ManagedArtifactRetriever for FixtureRetriever {
    fn retrieve(
        &self,
        request: ArtifactRequest<'_>,
        _cancelled: &AtomicBool,
    ) -> Result<RetrievedArtifact, ManagedInstallError> {
        // The bounds are the shared capability's, but the declaration is the registry's, and a
        // distribution that declared none would download without a ceiling.
        assert!(
            request.policy.permits_url(request.url),
            "the retriever was handed a url its own policy refuses"
        );
        self.requested
            .lock()
            .expect("requested")
            .push(request.url.to_owned());

        let directory = tempfile::tempdir().expect("artifact directory");
        let path = directory.path().join(request.file_name);
        std::fs::write(&path, &self.archive).expect("write fixture archive");
        Ok(RetrievedArtifact {
            path,
            _directory: directory,
        })
    }
}

/// A tar.gz holding a minimal jdtls-shaped layout.
fn jdtls_archive() -> Vec<u8> {
    let mut tar_bytes = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_bytes);
        for (name, bytes) in [
            (
                "plugins/org.eclipse.equinox.launcher_1.6.500.jar",
                b"jar" as &[u8],
            ),
            ("config_linux/config.ini", b"ini"),
            ("config_win/config.ini", b"ini"),
            ("config_mac/config.ini", b"ini"),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            let raw = name.as_bytes();
            header.as_old_mut().name[..raw.len()].copy_from_slice(raw);
            header.set_cksum();
            builder.append(&header, bytes).expect("append entry");
        }
        builder.finish().expect("finish tar");
    }
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &tar_bytes).expect("gzip");
    encoder.finish().expect("finish gzip")
}

fn retriever(archive: Vec<u8>) -> FixtureRetriever {
    FixtureRetriever {
        archive,
        requested: std::sync::Mutex::default(),
    }
}

fn never() -> AtomicBool {
    AtomicBool::new(false)
}

#[test]
fn a_declared_distribution_installs_into_a_directory_vanehub_owns() {
    let data = tempfile::tempdir().expect("data directory");
    let retriever = retriever(jdtls_archive());

    let installed = install_managed_server(&retriever, data.path(), registry::java(), &never())
        .expect("install");

    assert_eq!(
        installed,
        managed_install_directory(data.path(), "java"),
        "the install must land where discovery will look for it"
    );
    assert!(installed
        .join("plugins/org.eclipse.equinox.launcher_1.6.500.jar")
        .is_file());
    assert_eq!(
        retriever.requested.lock().expect("requested").len(),
        1,
        "one download, not one per entry"
    );
}

#[test]
fn a_language_with_no_declared_distribution_is_refused() {
    let data = tempfile::tempdir().expect("data directory");
    let retriever = retriever(jdtls_archive());

    let error = install_managed_server(&retriever, data.path(), registry::rust(), &never())
        .expect_err("refused");

    assert!(matches!(error, ManagedInstallError::Refused(_)));
    // Refused before anything was fetched: the five languages that install with a package manager
    // must not acquire a second install path by accident.
    assert!(retriever.requested.lock().expect("requested").is_empty());
    assert!(managed_install(data.path(), "rust").is_none());
}

#[test]
fn a_failed_install_leaves_nothing_that_looks_installed() {
    let data = tempfile::tempdir().expect("data directory");
    // Not a gzip stream at all, so extraction refuses before anything is placed.
    let retriever = retriever(b"not an archive".to_vec());

    assert!(install_managed_server(&retriever, data.path(), registry::java(), &never()).is_err());

    // The half state is the dangerous one: a directory that exists but holds no launcher reads as
    // installed to discovery and then fails at launch.
    assert!(managed_install(data.path(), "java").is_none());
}

#[test]
fn a_reinstall_replaces_rather_than_merges() {
    let data = tempfile::tempdir().expect("data directory");
    let retriever = retriever(jdtls_archive());
    let installed = install_managed_server(&retriever, data.path(), registry::java(), &never())
        .expect("install");
    // A launcher from an imaginary earlier version. Left in place, the glob would find two and
    // discovery would refuse the install outright.
    std::fs::write(
        installed.join("plugins/org.eclipse.equinox.launcher_1.0.0.jar"),
        b"stale",
    )
    .expect("stale launcher");

    install_managed_server(&retriever, data.path(), registry::java(), &never()).expect("reinstall");

    assert!(!installed
        .join("plugins/org.eclipse.equinox.launcher_1.0.0.jar")
        .exists());
    assert!(installed
        .join("plugins/org.eclipse.equinox.launcher_1.6.500.jar")
        .is_file());
}

#[test]
fn uninstall_removes_the_managed_directory_and_is_uneventful_when_there_is_none() {
    let data = tempfile::tempdir().expect("data directory");
    let retriever = retriever(jdtls_archive());
    install_managed_server(&retriever, data.path(), registry::java(), &never()).expect("install");
    assert!(managed_install(data.path(), "java").is_some());

    uninstall_managed_server(data.path(), "java").expect("uninstall");
    assert!(managed_install(data.path(), "java").is_none());

    // Called for a language that never had one, which is what happens when a user clicks
    // uninstall twice.
    uninstall_managed_server(data.path(), "java").expect("second uninstall");
}

#[test]
fn uninstall_never_touches_a_directory_the_user_pointed_at() {
    let data = tempfile::tempdir().expect("data directory");
    // The user's own jdtls, outside anything VaneHub created.
    let theirs = tempfile::tempdir().expect("user directory");
    std::fs::create_dir_all(theirs.path().join("plugins")).expect("their plugins");
    std::fs::write(
        theirs
            .path()
            .join("plugins/org.eclipse.equinox.launcher_9.9.9.jar"),
        b"theirs",
    )
    .expect("their launcher");

    uninstall_managed_server(data.path(), "java").expect("uninstall");

    // Losing someone's own install by clicking a button about VaneHub's copy is the failure this
    // asserts against.
    assert!(theirs
        .path()
        .join("plugins/org.eclipse.equinox.launcher_9.9.9.jar")
        .is_file());
}

#[test]
fn the_declared_distribution_states_whether_its_bytes_are_verified() {
    let distribution = registry::java()
        .distribution
        .as_ref()
        .expect("java declares a distribution");

    // Currently false, and the surface has to say so. If a digest is ever pinned this flips and
    // the wording follows it, rather than the wording being maintained separately.
    assert!(!distribution.is_verified());
    assert!(distribution.policy.is_bounded());
    assert!(distribution.policy.permits_url(distribution.url));
}
