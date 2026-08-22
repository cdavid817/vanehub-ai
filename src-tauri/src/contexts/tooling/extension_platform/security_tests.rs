//! The security suite for extension packages, in one place so it can be found.
//!
//! The per-module tests check that each rule works. These check the things that only show up when
//! the pieces are put together against real storage: that no hostile input reaches a panic, that a
//! genuine signature cannot be moved onto other bytes, that two installs racing leave one
//! consistent outcome, and that a half-written package leaves nothing behind.
//!
//! **What this is not.** There is no continuous fuzzing harness here. `cargo-fuzz` needs a nightly
//! toolchain, a separate workspace member, and a CI job, none of which exist; adding them is its
//! own piece of work. What is here instead is a *deterministic mutation corpus*: a valid package
//! truncated at every boundary and with every structurally significant header field corrupted, run
//! through the real reader. That is the part of fuzzing that pays for a bounded parser — it finds
//! the input that panics — and it is reproducible, which a fuzzer is not. It is called a corpus
//! rather than a fuzzer because that is what it is.

use crate::contexts::tooling::extension_platform::application::{
    PackageVerificationService, PublisherKeyDirectory, SnapshotContentStore,
    SnapshotPointerRepository,
};
use crate::contexts::tooling::extension_platform::domain::{
    parse_signature_envelope, signed_payload, ExtensionId, InstallationId, ManifestDigest,
    PackageFacts, PackageHash, PackageSignature, PortablePackagePath, PublisherId,
    PublisherKeyFingerprint, PublisherKeyRecord, PublisherPublicKey, PublisherTrustState,
    SignatureAlgorithm, SignatureEnvelope, SignatureRejection, SignatureState, SnapshotId,
    SnapshotRecord, DEFAULT_EXTENSION_PACKAGE_LIMITS, PACKAGE_MANIFEST_ENTRY, PUBLISHER_KEY_BYTES,
    SIGNATURE_BYTES,
};
use crate::contexts::tooling::extension_platform::infrastructure::{
    read_extension_package, ExtensionRoots, FilesystemSnapshotContentStore,
    SqliteSnapshotPointerRepository,
};
use crate::platform::content_address::sha256_hex;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use semver::Version;
use std::io::{Cursor, Write};
use std::sync::Arc;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const MANIFEST: &str = "\
schema_version: 1
id: acme.git-guardian
display_name: Git Guardian
publisher: acme
version: 1.2.0
min_vanehub_version: \">=0.9.0\"
runtime:
  kind: wasm-module
  entry: runtime/guardian.wasm
  trust_profile: strict
";

fn valid_package() -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, content) in [
        (PACKAGE_MANIFEST_ENTRY, MANIFEST.as_bytes()),
        ("runtime/guardian.wasm", b"\0asm\x01\0\0\0".as_slice()),
        ("README.md", b"# Git Guardian\n".as_slice()),
    ] {
        writer.start_file(name, options).expect("start ZIP entry");
        writer.write_all(content).expect("write ZIP entry");
    }
    writer.finish().expect("finish ZIP").into_inner()
}

fn read(bytes: &[u8], label: &str) -> bool {
    let home = TempDirectory::new(label);
    let staging = home.path().join("staging");
    let outcome = read_extension_package(
        bytes,
        &staging,
        &Version::parse("1.0.0").expect("version"),
        DEFAULT_EXTENSION_PACKAGE_LIMITS,
    );
    assert!(
        !staging.exists(),
        "staging must not survive any outcome: {label}"
    );
    outcome.is_ok()
}

// ---------------------------------------------------------------------------
// Archive parser boundaries: the deterministic mutation corpus
// ---------------------------------------------------------------------------

#[test]
fn no_truncation_of_a_valid_package_reaches_a_panic() {
    // Every prefix of a well-formed archive, at a stride that covers the end record, the central
    // directory, and the middle of a compressed stream. A truncated archive is a thing that
    // actually happens -- an interrupted download -- and the only unacceptable answer is a crash.
    let package = valid_package();
    for length in (0..package.len()).step_by(7) {
        read(&package[..length], "corpus-truncated");
    }
}

#[test]
fn no_single_byte_corruption_of_a_valid_package_reaches_a_panic() {
    // One flipped byte at every offset. Structurally significant fields -- signatures, lengths,
    // offsets, the compressed stream itself -- are all covered without having to enumerate them.
    let package = valid_package();
    for offset in 0..package.len() {
        let mut corrupted = package.clone();
        corrupted[offset] ^= 0xff;
        read(&corrupted, "corpus-corrupted");
    }
}

#[test]
fn no_repeated_or_appended_structure_reaches_a_panic() {
    let package = valid_package();
    let cases: Vec<Vec<u8>> = vec![
        // An archive inside a prefix, which is how a self-extracting stub is built.
        [b"MZ".to_vec(), package.clone()].concat(),
        [package.clone(), b"trailing".to_vec()].concat(),
        // Nothing at all, and something that is not an archive.
        Vec::new(),
        b"not an archive".to_vec(),
        vec![0_u8; 1_024],
    ];

    for (index, case) in cases.iter().enumerate() {
        assert!(
            !read(case, &format!("corpus-structure-{index}")),
            "case {index} must not be admitted"
        );
    }
}

#[test]
fn an_archive_appended_to_itself_is_read_as_one_and_is_not_the_same_package() {
    // Recorded as behavior, not as a guarantee. Two byte-identical archives concatenated leave a
    // final end record that is genuinely the last thing in the file, and whose central-directory
    // offset happens to point at the first copy's -- so the reader sees one valid archive and the
    // trailing-data and prefix checks have nothing to object to.
    let package = valid_package();
    let doubled = [package.clone(), package.clone()].concat();
    assert!(read(&doubled, "corpus-doubled"));

    // What actually stops this being useful to an attacker is the hash: the signature covers every
    // byte of the file, so appending anything at all produces a package no signature attests to.
    assert_ne!(sha256_hex(&doubled), sha256_hex(&package));
    let service = PackageVerificationService::new(Arc::new(FixedDirectory(Some(key(
        PublisherTrustState::Trusted,
    )))));
    assert_eq!(
        service
            .verify(Some(&envelope_for(&package)), &facts(&doubled))
            .expect("lookup"),
        SignatureState::Rejected(SignatureRejection::PackageHashMismatch)
    );
}

// ---------------------------------------------------------------------------
// Path normalization
// ---------------------------------------------------------------------------

#[test]
fn no_hostile_entry_name_is_admitted_by_the_declared_path_rule() {
    // A corpus rather than a sample. Each of these is a way a real extractor has been made to
    // write outside its destination.
    let hostile = [
        "../escape",
        "../../escape",
        "a/../../escape",
        "/absolute",
        "//server/share",
        "\\\\server\\share",
        "C:/windows",
        "C:windows",
        "a\\b",
        "a/./b",
        "a//b",
        "a/",
        ".",
        "..",
        "",
        "CON",
        "con.txt",
        "AUX",
        "NUL",
        "COM1",
        "LPT9",
        "trailing.",
        "trailing ",
        "stream:$DATA",
        "nul\u{0}byte",
        "bell\u{7}",
        "\u{202e}gnp.exe",
    ];

    for name in hostile {
        assert!(
            PortablePackagePath::parse(name).is_err(),
            "{name:?} must not be a declared package path"
        );
    }
}

// ---------------------------------------------------------------------------
// Signature substitution and revoked keys
// ---------------------------------------------------------------------------

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[11_u8; PUBLISHER_KEY_BYTES])
}

fn key(trust_state: PublisherTrustState) -> PublisherKeyRecord {
    PublisherKeyRecord {
        publisher: PublisherId::parse("acme").expect("publisher"),
        key: PublisherPublicKey::from_bytes(signing_key().verifying_key().to_bytes()),
        trust_state,
    }
}

fn envelope_for(package: &[u8]) -> Vec<u8> {
    let digest = sha256_hex(package);
    let mut envelope = SignatureEnvelope {
        envelope_version: 1,
        algorithm: SignatureAlgorithm::Ed25519,
        publisher: PublisherId::parse("acme").expect("publisher"),
        extension: ExtensionId::parse("acme.git-guardian").expect("extension"),
        version: Version::parse("1.2.0").expect("version"),
        package_hash: PackageHash::parse(&digest).expect("hash"),
        package_bytes: package.len() as u64,
        claimed_manifest_digest: ManifestDigest::parse(&sha256_hex(MANIFEST.as_bytes()))
            .expect("digest"),
        key_fingerprint: key(PublisherTrustState::Trusted).fingerprint(),
        signature: PackageSignature::from_bytes([0_u8; SIGNATURE_BYTES]),
    };
    envelope.signature =
        PackageSignature::from_bytes(signing_key().sign(&signed_payload(&envelope)).to_bytes());

    format!(
        "envelope_version: 1\nalgorithm: ed25519\npublisher: acme\nextension: acme.git-guardian\n\
         version: 1.2.0\npackage_sha256: {digest}\npackage_bytes: {}\nmanifest_sha256: {}\n\
         key_fingerprint: {}\nsignature: {}\n",
        package.len(),
        envelope.claimed_manifest_digest.as_str(),
        envelope.key_fingerprint.as_str(),
        STANDARD.encode(envelope.signature.as_bytes())
    )
    .into_bytes()
}

struct FixedDirectory(Option<PublisherKeyRecord>);

impl PublisherKeyDirectory for FixedDirectory {
    fn find(
        &self,
        _fingerprint: &PublisherKeyFingerprint,
    ) -> Result<Option<PublisherKeyRecord>, String> {
        Ok(self.0.clone())
    }
}

fn facts(package: &[u8]) -> PackageFacts {
    PackageFacts {
        hash: PackageHash::parse(&sha256_hex(package)).expect("hash"),
        byte_length: package.len() as u64,
    }
}

#[test]
fn a_genuine_signature_cannot_be_moved_onto_other_bytes() {
    // The attack the whole design is aimed at: a real publisher's real signature, presented
    // alongside a package they did not sign.
    let signed = valid_package();
    let envelope = envelope_for(&signed);
    let service = PackageVerificationService::new(Arc::new(FixedDirectory(Some(key(
        PublisherTrustState::Trusted,
    )))));

    assert!(
        service
            .verify(Some(&envelope), &facts(&signed))
            .expect("lookup")
            .verified()
            .is_some(),
        "the fixture must verify against its own bytes"
    );

    let mut other = signed.clone();
    other.extend_from_slice(b"one more byte");
    assert_eq!(
        service
            .verify(Some(&envelope), &facts(&other))
            .expect("lookup"),
        SignatureState::Rejected(SignatureRejection::PackageHashMismatch)
    );
}

#[test]
fn a_revoked_key_stops_authorizing_without_the_signature_changing() {
    let signed = valid_package();
    let envelope = envelope_for(&signed);

    let trusted = PackageVerificationService::new(Arc::new(FixedDirectory(Some(key(
        PublisherTrustState::Trusted,
    )))));
    assert!(trusted
        .verify(Some(&envelope), &facts(&signed))
        .expect("lookup")
        .verified()
        .is_some());

    let revoked = PackageVerificationService::new(Arc::new(FixedDirectory(Some(key(
        PublisherTrustState::Revoked,
    )))));
    assert_eq!(
        revoked
            .verify(Some(&envelope), &facts(&signed))
            .expect("lookup"),
        SignatureState::Rejected(SignatureRejection::RevokedPublisherKey)
    );
}

#[test]
fn an_envelope_from_the_corpus_never_reaches_a_panic() {
    let signed = valid_package();
    let envelope = envelope_for(&signed);
    let service = PackageVerificationService::new(Arc::new(FixedDirectory(Some(key(
        PublisherTrustState::Trusted,
    )))));

    let original = parse_signature_envelope(&envelope).expect("the fixture must parse");
    let verify_mutation = |bytes: &[u8], label: String| {
        let Ok(state) = service.verify(Some(bytes), &facts(&signed)) else {
            return;
        };
        if state.verified().is_none() {
            return;
        }
        // A mutation is allowed to verify only when it still decodes to the same envelope --
        // dropping a trailing newline changes no value, and the signature covers values rather
        // than bytes. Anything else verifying would mean a changed claim still attested to.
        assert_eq!(
            parse_signature_envelope(bytes).ok().as_ref(),
            Some(&original),
            "{label} verified while saying something different"
        );
    };

    for offset in 0..envelope.len() {
        let mut corrupted = envelope.clone();
        corrupted[offset] ^= 0xff;
        verify_mutation(&corrupted, format!("corruption at {offset}"));
    }
    for length in 0..envelope.len() {
        verify_mutation(&envelope[..length], format!("truncation to {length}"));
    }
}

// ---------------------------------------------------------------------------
// Concurrent installs and partial writes
// ---------------------------------------------------------------------------

struct Storage {
    _directory: TempDirectory,
    database: Arc<NativeDatabase>,
    roots: ExtensionRoots,
}

fn storage(label: &str) -> Storage {
    let directory = TempDirectory::new(label);
    let database = Arc::new(
        NativeDatabase::new(directory.path().to_path_buf()).expect("database should open"),
    );
    let roots = ExtensionRoots::new(directory.path().join("extensions"));
    roots.prepare().expect("roots");
    Storage {
        _directory: directory,
        database,
        roots,
    }
}

fn staged_content(roots: &ExtensionRoots, name: &str) -> std::path::PathBuf {
    let path = roots
        .root(crate::contexts::tooling::extension_platform::domain::ExtensionRootScope::Quarantine)
        .join(name);
    roots.create(&path).expect("staged");
    std::fs::write(path.join("content.bin"), b"identical bytes").expect("write");
    path
}

#[test]
fn two_installs_of_the_same_package_leave_one_copy_and_both_succeed() {
    // Content is addressed by its own digest, so the loser of the race finds exactly the bytes it
    // was about to write. Neither install has anything to complain about.
    let storage = storage("security-concurrent-same");
    let store = Arc::new(FilesystemSnapshotContentStore::new(storage.roots.clone()));
    let hash = PackageHash::parse(&"a".repeat(64)).expect("hash");
    let first = staged_content(&storage.roots, "operation-1");
    let second = staged_content(&storage.roots, "operation-2");

    let one = Arc::clone(&store);
    let two = Arc::clone(&store);
    let hash_one = hash.clone();
    let hash_two = hash.clone();
    let left = std::thread::spawn(move || one.publish(&first, &hash_one));
    let right = std::thread::spawn(move || two.publish(&second, &hash_two));

    assert!(left.join().expect("thread").is_ok());
    assert!(right.join().expect("thread").is_ok());

    let destination = storage.roots.package(&hash).expect("package path");
    assert_eq!(
        std::fs::read(destination.join("content.bin")).expect("content"),
        b"identical bytes"
    );
    assert_eq!(
        std::fs::read_dir(storage.roots.root(
            crate::contexts::tooling::extension_platform::domain::ExtensionRootScope::Quarantine
        ))
        .expect("quarantine")
        .count(),
        0,
        "both staged copies are gone, whichever one was renamed"
    );
}

#[test]
fn two_installs_of_different_versions_produce_exactly_one_winner() {
    // The pointer is guarded by a revision, so one of the two writes is refused. Which one wins is
    // not the property being asserted; that exactly one does, and that the pointer is readable
    // afterwards, is.
    let storage = storage("security-concurrent-pointer");
    let pointers = Arc::new(SqliteSnapshotPointerRepository::new(
        storage.database.clone(),
        InstallationId::parse("install-1").expect("installation"),
    ));

    let record = |snapshot: &str, hash: &str| SnapshotRecord {
        snapshot: SnapshotId::parse(snapshot).expect("snapshot"),
        extension: ExtensionId::parse("acme.git-guardian").expect("extension"),
        version: Version::parse("1.2.0").expect("version"),
        package_hash: PackageHash::parse(hash).expect("hash"),
        manifest_digest: ManifestDigest::parse(&"b".repeat(64)).expect("digest"),
        created_at: "2026-08-22T00:00:00Z".to_string(),
    };

    let one = Arc::clone(&pointers);
    let two = Arc::clone(&pointers);
    let left = std::thread::spawn(move || one.point_at(&record("snapshot-1", &"a".repeat(64)), 0));
    let right = std::thread::spawn(move || two.point_at(&record("snapshot-2", &"c".repeat(64)), 0));

    let outcomes = [left.join().expect("thread"), right.join().expect("thread")];
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_ok()).count(),
        1,
        "exactly one write may land: {outcomes:?}"
    );

    let active = pointers
        .active(&ExtensionId::parse("acme.git-guardian").expect("extension"))
        .expect("active")
        .expect("pointer");
    assert_eq!(active.revision, 1);
    assert!(["snapshot-1", "snapshot-2"].contains(&active.active.as_str()));
}

#[test]
fn a_package_refused_after_its_files_were_written_leaves_nothing_behind() {
    // The ordinary shape of a failed install: extraction succeeds, and the package is refused
    // afterwards. What must not survive is a staging directory holding those files, because the
    // next attempt would inherit them. `read` asserts the directory is gone on every path.
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, content) in [
        // Parses as a document, and is missing every field after the first.
        (PACKAGE_MANIFEST_ENTRY, b"schema_version: 1\n".as_slice()),
        ("runtime/guardian.wasm", b"\0asm\x01\0\0\0".as_slice()),
        ("assets/icon.png", b"\x89PNG\r\n\x1a\n".as_slice()),
    ] {
        writer.start_file(name, options).expect("start ZIP entry");
        writer.write_all(content).expect("write ZIP entry");
    }
    let package = writer.finish().expect("finish ZIP").into_inner();

    assert!(!read(&package, "security-partial-write"));
}

#[test]
fn a_destination_that_already_holds_a_file_is_refused_rather_than_overwritten() {
    let storage = storage("security-partial-destination");
    let staging = storage
        .roots
        .root(crate::contexts::tooling::extension_platform::domain::ExtensionRootScope::Quarantine)
        .join("operation-1");
    storage.roots.create(&staging).expect("staging");
    std::fs::create_dir_all(staging.join("runtime")).expect("runtime directory");
    std::fs::write(staging.join("runtime/guardian.wasm"), b"an earlier attempt")
        .expect("leftover file");

    let outcome = read_extension_package(
        &valid_package(),
        &staging,
        &Version::parse("1.0.0").expect("version"),
        DEFAULT_EXTENSION_PACKAGE_LIMITS,
    );

    assert!(
        outcome.is_err(),
        "an existing staging directory is refused, not written into"
    );
    assert_eq!(
        std::fs::read(staging.join("runtime/guardian.wasm")).expect("leftover"),
        b"an earlier attempt",
        "and the earlier attempt is left exactly as it was"
    );
}
