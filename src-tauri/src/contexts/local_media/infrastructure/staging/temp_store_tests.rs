use super::*;
use crate::contexts::local_media::domain::{LocalMediaErrorCode, STAGED_INPUT_TTL_MS};
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;

struct SequentialIds {
    counter: AtomicU64,
}

impl OpaqueIdFactory for SequentialIds {
    fn next(&self, prefix: &str) -> String {
        format!(
            "{prefix}{:032x}",
            self.counter.fetch_add(1, Ordering::SeqCst)
        )
    }
}

struct MovableClock {
    millis: AtomicU64,
}

impl LocalMediaClock for MovableClock {
    fn now_iso(&self) -> String {
        "2026-01-01T00:00:00Z".to_string()
    }

    fn now_ms(&self) -> u64 {
        self.millis.load(Ordering::SeqCst)
    }
}

struct Fixture {
    _root: TempDir,
    source_dir: TempDir,
    store: FilesystemMediaTempStore,
    clock: Arc<MovableClock>,
}

fn fixture() -> Fixture {
    let root = TempDir::new().expect("root");
    let source_dir = TempDir::new().expect("sources");
    let clock = Arc::new(MovableClock {
        millis: AtomicU64::new(1_000),
    });
    let store = FilesystemMediaTempStore::new(
        root.path().to_path_buf(),
        Arc::new(SequentialIds {
            counter: AtomicU64::new(0),
        }),
        clock.clone(),
        AdmissionLimits::HARD_CEILING,
    );
    Fixture {
        _root: root,
        source_dir,
        store,
        clock,
    }
}

fn png_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR".to_vec();
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
    bytes
}

fn write_source(fixture: &Fixture, name: &str, bytes: &[u8]) -> PathBuf {
    let path = fixture.source_dir.path().join(name);
    fs::write(&path, bytes).expect("write source");
    path
}

#[test]
fn a_valid_image_is_copied_into_staging_and_described_without_its_path() {
    let fixture = fixture();
    let source = write_source(&fixture, "scan.png", &png_bytes(100, 100));

    let staged = fixture.store.stage_ocr_source(&source).expect("stage");
    assert_eq!(staged.display_name, "scan.png");
    assert_eq!(staged.media_type, OcrMediaType::Image);
    assert_eq!(staged.byte_length, png_bytes(100, 100).len() as u64);
    assert!(StagedInputId::parse(staged.staged_input_id.as_str()).is_some());

    let claimed = fixture.store.claim(&staged.staged_input_id).expect("claim");
    assert!(claimed.path.exists());
    assert_ne!(
        claimed.path, source,
        "the worker must not receive the caller's path"
    );
    assert_eq!(
        fs::read(&claimed.path).expect("read staged"),
        png_bytes(100, 100)
    );
}

#[test]
fn the_staged_file_name_is_opaque_and_not_the_users() {
    let fixture = fixture();
    let source = write_source(&fixture, "quarterly-earnings-draft.png", &png_bytes(10, 10));
    let staged = fixture.store.stage_ocr_source(&source).expect("stage");
    let claimed = fixture.store.claim(&staged.staged_input_id).expect("claim");
    let rendered = claimed.path.to_string_lossy().to_string();
    assert!(!rendered.contains("quarterly-earnings-draft"));
}

#[test]
fn a_misleading_extension_is_rejected_by_content() {
    let fixture = fixture();
    let source = write_source(&fixture, "not-really.png", b"GIF89a\x01\x00\x01\x00");
    let error = fixture
        .store
        .stage_ocr_source(&source)
        .expect_err("must reject");
    assert_eq!(error.code(), LocalMediaErrorCode::UnsupportedMediaType);
}

#[test]
fn an_executable_renamed_to_png_is_rejected() {
    let fixture = fixture();
    let source = write_source(&fixture, "payload.png", b"MZ\x90\x00\x03\x00\x00\x00");
    assert_eq!(
        fixture
            .store
            .stage_ocr_source(&source)
            .map(|_| ())
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::UnsupportedMediaType)
    );
}

#[test]
fn an_oversized_file_is_rejected_before_it_is_copied() {
    let root = TempDir::new().expect("root");
    let source_dir = TempDir::new().expect("sources");
    let store = FilesystemMediaTempStore::new(
        root.path().to_path_buf(),
        Arc::new(SequentialIds {
            counter: AtomicU64::new(0),
        }),
        Arc::new(MovableClock {
            millis: AtomicU64::new(0),
        }),
        AdmissionLimits {
            max_input_bytes: 16,
            ..AdmissionLimits::HARD_CEILING
        },
    );
    let source = source_dir.path().join("big.png");
    fs::write(&source, png_bytes(10, 10)).expect("write");

    let error = store.stage_ocr_source(&source).expect_err("too large");
    assert_eq!(error.code(), LocalMediaErrorCode::InputTooLarge);
    assert!(
        !root.path().join("staging").exists()
            || fs::read_dir(root.path().join("staging"))
                .expect("read")
                .next()
                .is_none(),
        "nothing may be copied for a rejected file"
    );
}

#[test]
fn a_pixel_bomb_is_rejected_on_its_declared_dimensions() {
    let fixture = fixture();
    // 70,000 x 70,000 declares 4.9 billion pixels in 33 bytes of header.
    let source = write_source(&fixture, "bomb.png", &png_bytes(70_000, 70_000));
    let error = fixture
        .store
        .stage_ocr_source(&source)
        .expect_err("pixel bomb");
    assert_eq!(error.code(), LocalMediaErrorCode::ImagePixelLimitExceeded);
}

#[test]
fn an_image_whose_dimensions_cannot_be_read_is_refused() {
    let fixture = fixture();
    let source = write_source(
        &fixture,
        "truncated.png",
        b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR",
    );
    let error = fixture
        .store
        .stage_ocr_source(&source)
        .expect_err("unreadable header");
    assert_eq!(error.code(), LocalMediaErrorCode::UnsupportedMediaType);
}

#[test]
fn a_pdf_is_admitted_without_a_pixel_check() {
    let fixture = fixture();
    let source = write_source(&fixture, "report.pdf", b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n");
    let staged = fixture.store.stage_ocr_source(&source).expect("stage pdf");
    assert_eq!(staged.media_type, OcrMediaType::Pdf);
}

#[test]
fn a_directory_is_not_a_regular_file() {
    let fixture = fixture();
    let directory = fixture.source_dir.path().join("folder");
    fs::create_dir(&directory).expect("mkdir");
    let error = fixture
        .store
        .stage_ocr_source(&directory)
        .expect_err("directory");
    assert_eq!(error.code(), LocalMediaErrorCode::UnsupportedMediaType);
}

#[test]
fn a_missing_file_reports_input_not_found() {
    let fixture = fixture();
    let missing = fixture.source_dir.path().join("absent.png");
    let error = fixture
        .store
        .stage_ocr_source(&missing)
        .expect_err("missing");
    assert_eq!(error.code(), LocalMediaErrorCode::InputNotFound);
}

#[test]
fn a_relative_path_is_refused_without_touching_the_filesystem() {
    let fixture = fixture();
    let error = fixture
        .store
        .stage_ocr_source(Path::new("scan.png"))
        .expect_err("relative");
    assert_eq!(error.code(), LocalMediaErrorCode::InputNotFound);
}

#[cfg(unix)]
#[test]
fn a_symlink_source_is_refused_even_when_its_target_is_valid() {
    let fixture = fixture();
    let real = write_source(&fixture, "real.png", &png_bytes(10, 10));
    let link = fixture.source_dir.path().join("link.png");
    std::os::unix::fs::symlink(&real, &link).expect("symlink");

    let error = fixture
        .store
        .stage_ocr_source(&link)
        .expect_err("symlink source");
    assert_eq!(error.code(), LocalMediaErrorCode::UnsupportedMediaType);
}

#[cfg(unix)]
#[test]
fn a_fifo_is_refused_rather_than_read() {
    // Reading a FIFO would block the staging call indefinitely.
    use std::ffi::CString;
    let fixture = fixture();
    let fifo = fixture.source_dir.path().join("pipe.png");
    let Ok(raw) = CString::new(fifo.to_string_lossy().as_bytes()) else {
        return;
    };
    let made = unsafe { libc::mkfifo(raw.as_ptr(), 0o600) };
    if made != 0 {
        return;
    }
    let error = fixture.store.stage_ocr_source(&fifo).expect_err("fifo");
    assert_eq!(error.code(), LocalMediaErrorCode::UnsupportedMediaType);
}

#[cfg(unix)]
#[test]
fn staged_directories_and_files_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = fixture();
    let source = write_source(&fixture, "scan.png", &png_bytes(10, 10));
    let staged = fixture.store.stage_ocr_source(&source).expect("stage");
    let claimed = fixture.store.claim(&staged.staged_input_id).expect("claim");

    let file_mode = fs::metadata(&claimed.path)
        .expect("meta")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(
        file_mode, 0o600,
        "staged media must not be group or world readable"
    );
    let parent = claimed.path.parent().expect("parent");
    let dir_mode = fs::metadata(parent).expect("meta").permissions().mode() & 0o777;
    assert_eq!(dir_mode, 0o700);
}

#[test]
fn a_staged_input_can_only_be_claimed_once() {
    let fixture = fixture();
    let source = write_source(&fixture, "scan.png", &png_bytes(10, 10));
    let staged = fixture.store.stage_ocr_source(&source).expect("stage");

    assert!(fixture.store.claim(&staged.staged_input_id).is_ok());
    assert_eq!(
        fixture
            .store
            .claim(&staged.staged_input_id)
            .map(|_| ())
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::InputNotFound)
    );
}

#[test]
fn an_unknown_or_malformed_staged_id_cannot_reach_the_filesystem() {
    let fixture = fixture();
    for hostile in [
        "lmi-../../../etc/passwd",
        "lmi-",
        "../staging",
        "lmi-not-hex",
    ] {
        let id = StagedInputId::new(hostile);
        assert_eq!(
            fixture
                .store
                .claim(&id)
                .map(|_| ())
                .map_err(|error| error.code()),
            Err(LocalMediaErrorCode::InputNotFound),
            "{hostile} must not resolve"
        );
    }
}

#[test]
fn an_expired_staged_input_cannot_be_claimed() {
    let fixture = fixture();
    let source = write_source(&fixture, "scan.png", &png_bytes(10, 10));
    let staged = fixture.store.stage_ocr_source(&source).expect("stage");

    fixture
        .clock
        .millis
        .store(1_000 + STAGED_INPUT_TTL_MS, Ordering::SeqCst);
    assert_eq!(
        fixture
            .store
            .claim(&staged.staged_input_id)
            .map(|_| ())
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::InputNotFound)
    );
}

#[test]
fn cleanup_removes_the_staged_directory_and_is_idempotent() {
    let fixture = fixture();
    let source = write_source(&fixture, "scan.png", &png_bytes(10, 10));
    let staged = fixture.store.stage_ocr_source(&source).expect("stage");
    let claimed = fixture.store.claim(&staged.staged_input_id).expect("claim");
    let directory = claimed.path.parent().expect("parent").to_path_buf();

    fixture.store.cleanup_staged(&staged.staged_input_id);
    assert!(!directory.exists());
    fixture.store.cleanup_staged(&staged.staged_input_id);
}

#[test]
fn staging_bytes_never_consults_a_host_path() {
    let fixture = fixture();
    let staged = fixture
        .store
        .stage_bytes(b"%PDF-1.7\n1 0 obj\n", "artifact.pdf")
        .expect("stage bytes");
    assert_eq!(staged.media_type, OcrMediaType::Pdf);
    assert_eq!(staged.display_name, "artifact.pdf");
}

#[test]
fn staging_bytes_applies_the_same_content_and_size_checks() {
    let fixture = fixture();
    assert_eq!(
        fixture
            .store
            .stage_bytes(b"GIF89a", "sneaky.pdf")
            .map(|_| ())
            .map_err(|e| e.code()),
        Err(LocalMediaErrorCode::UnsupportedMediaType)
    );
    assert_eq!(
        fixture
            .store
            .stage_bytes(&png_bytes(70_000, 70_000), "bomb.png")
            .map(|_| ())
            .map_err(|e| e.code()),
        Err(LocalMediaErrorCode::ImagePixelLimitExceeded)
    );
}

#[test]
fn an_authorized_output_is_the_only_playable_path() {
    let fixture = fixture();
    let authorized = fixture
        .store
        .authorize_output_wav("operation-1")
        .expect("authorize");
    fs::write(&authorized, wav_bytes()).expect("write wav");

    assert!(fixture
        .store
        .verify_output_wav("operation-1", &authorized)
        .is_ok());
    assert_eq!(
        fixture
            .store
            .verify_output_wav("operation-1", Path::new("/etc/passwd"))
            .map(|_| ())
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::WorkerProtocolError)
    );
    assert_eq!(
        fixture
            .store
            .verify_output_wav("operation-2", &authorized)
            .map(|_| ())
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::WorkerProtocolError)
    );
}

#[test]
fn a_non_wav_at_the_authorized_path_is_refused() {
    let fixture = fixture();
    let authorized = fixture
        .store
        .authorize_output_wav("operation-1")
        .expect("authorize");
    fs::write(&authorized, b"not audio at all").expect("write");
    assert_eq!(
        fixture
            .store
            .verify_output_wav("operation-1", &authorized)
            .map(|_| ())
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::WorkerProtocolError)
    );
}

fn wav_bytes() -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&36u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&[1, 0, 1, 0]);
    bytes.extend_from_slice(&16_000u32.to_le_bytes());
    bytes.extend_from_slice(&32_000u32.to_le_bytes());
    bytes.extend_from_slice(&[2, 0, 16, 0]);
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes
}

#[test]
fn operation_cleanup_removes_the_whole_directory() {
    let fixture = fixture();
    let authorized = fixture
        .store
        .authorize_output_wav("operation-1")
        .expect("authorize");
    fs::write(&authorized, wav_bytes()).expect("write");
    let directory = authorized.parent().expect("parent").to_path_buf();

    fixture.store.cleanup_operation("operation-1");
    assert!(!directory.exists());
    fixture.store.cleanup_operation("operation-1");
}

#[test]
fn a_recording_path_is_reserved_and_cleaned_by_id() {
    let fixture = fixture();
    let recording = RecordingId::new("lmr-0123456789abcdef0123456789abcdef");
    let path = fixture
        .store
        .authorize_recording_wav(&recording)
        .expect("authorize");
    fs::write(&path, wav_bytes()).expect("write");
    assert!(path.exists());

    fixture.store.cleanup_recording(&recording);
    assert!(!path.exists());
}

#[test]
fn a_malformed_operation_id_cannot_escape_the_root() {
    let fixture = fixture();
    for hostile in ["../../etc", "..", "a/b", "a\\b", ""] {
        assert!(
            fixture.store.authorize_output_wav(hostile).is_err(),
            "{hostile} must not resolve to a path"
        );
    }
}

#[test]
fn the_stale_sweep_removes_only_entries_past_the_window() {
    let fixture = fixture();
    let fresh = fixture
        .store
        .authorize_output_wav("operation-fresh")
        .expect("fresh");
    fs::write(&fresh, wav_bytes()).expect("write");

    // Nothing is old yet.
    assert_eq!(fixture.store.sweep_stale(24 * 60 * 60 * 1000), 0);
    assert!(fresh.exists());

    // A zero window makes everything stale, which is the same code path an old entry takes.
    let removed = fixture.store.sweep_stale(0);
    assert!(removed >= 1);
    assert!(!fresh.exists());
}

#[test]
fn the_sweep_tolerates_a_missing_root() {
    let root = TempDir::new().expect("root");
    let path = root.path().join("never-created");
    let store = FilesystemMediaTempStore::new(
        path,
        Arc::new(SequentialIds {
            counter: AtomicU64::new(0),
        }),
        Arc::new(MovableClock {
            millis: AtomicU64::new(0),
        }),
        AdmissionLimits::HARD_CEILING,
    );
    assert_eq!(store.sweep_stale(0), 0);
}
