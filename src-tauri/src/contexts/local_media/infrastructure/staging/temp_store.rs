//! Filesystem-backed ephemeral media.
//!
//! Layout:
//!
//! ```text
//! <root>/staging/<staged-input-id>/source.<ext>
//! <root>/recordings/<recording-id>/input.wav
//! <root>/operations/<operation-id>/output.wav
//! ```
//!
//! Recordings get their own tier rather than living under `operations/` because a recording exists
//! before its transcription operation does -- capture starts on pointer-down, and the operation is
//! not allocated until release.
//!
//! Every directory name is an opaque id this context minted. That is the primary containment
//! mechanism: a name that cannot express a separator or a `..` cannot escape, and the canonical
//! prefix check below is the second line rather than the first.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use crate::contexts::local_media::application::ports::{
    ClaimedInput, LocalMediaClock, MediaTempStore, OpaqueIdFactory,
};
use crate::contexts::local_media::domain::{
    exceeds_pixel_limit, image_dimensions, sanitize_display_name, sniff_media, AdmissionLimits,
    LocalMediaError, LocalMediaErrorCode, OcrMediaType, RecordingId, SniffedFormat, StagedInputId,
    StagedInputRecord, StagedOcrSource,
};

const STAGING_DIR: &str = "staging";
const RECORDINGS_DIR: &str = "recordings";
const OPERATIONS_DIR: &str = "operations";
const OUTPUT_FILE: &str = "output.wav";
const RECORDING_FILE: &str = "input.wav";

/// Enough to reach a JPEG start-of-frame past a full EXIF thumbnail without reading a whole file
/// into memory just to learn its dimensions.
const HEADER_SCAN_BYTES: usize = 128 * 1024;
/// A generated utterance is seconds of 16-bit mono PCM; anything larger did not come from the
/// configured engine.
const MAX_OUTPUT_WAV_BYTES: u64 = 64 * 1024 * 1024;
/// A sweep that walks an unbounded number of entries at startup would delay the first window.
const MAX_SWEEP_ENTRIES: usize = 512;

pub(crate) struct FilesystemMediaTempStore {
    root: PathBuf,
    ids: Arc<dyn OpaqueIdFactory>,
    clock: Arc<dyn LocalMediaClock>,
    limits: AdmissionLimits,
    staged: Mutex<HashMap<String, StagedInputRecord>>,
}

impl FilesystemMediaTempStore {
    pub(crate) fn new(
        root: PathBuf,
        ids: Arc<dyn OpaqueIdFactory>,
        clock: Arc<dyn LocalMediaClock>,
        limits: AdmissionLimits,
    ) -> Self {
        Self {
            root,
            ids,
            clock,
            limits,
            staged: Mutex::new(HashMap::new()),
        }
    }

    fn storage_error() -> LocalMediaError {
        LocalMediaError::new(LocalMediaErrorCode::TempStorageFailed)
    }

    /// Create a directory tree with owner-only permissions.
    ///
    /// On Unix the mode is set after creation rather than through `DirBuilder::mode`, because the
    /// parents created along the way need it too and `create_dir_all` applies the mode only to the
    /// leaf.
    fn ensure_directory(path: &Path) -> Result<(), LocalMediaError> {
        fs::create_dir_all(path).map_err(|_| Self::storage_error())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
        }
        Ok(())
    }

    fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), LocalMediaError> {
        fs::write(path, bytes).map_err(|_| Self::storage_error())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    /// Resolve `<root>/<tier>/<id>` for an id this context minted.
    ///
    /// The id is re-validated here even though it was validated at the boundary: this is the last
    /// point before a caller-influenced string becomes a path component.
    fn owned_directory(&self, tier: &str, id: &str) -> Result<PathBuf, LocalMediaError> {
        if !Self::is_opaque_component(id) {
            return Err(LocalMediaError::new(LocalMediaErrorCode::InputNotFound));
        }
        Ok(self.root.join(tier).join(id))
    }

    /// An id is a path component only if it is non-empty, bounded, and made of characters that
    /// cannot form a traversal or a drive-relative reference on any supported platform.
    fn is_opaque_component(id: &str) -> bool {
        !id.is_empty()
            && id.len() <= 96
            && id.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            })
    }

    /// Read a bounded prefix without following a link and without reading a non-regular file.
    fn read_admitted_prefix(source: &Path) -> Result<(Vec<u8>, u64), LocalMediaError> {
        if !source.is_absolute() {
            return Err(LocalMediaError::new(LocalMediaErrorCode::InputNotFound));
        }
        // `symlink_metadata` does not follow the link, so a symlink is visible as one. Following it
        // would let a picker result point anywhere the process can read.
        let link_metadata = fs::symlink_metadata(source).map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => {
                LocalMediaError::new(LocalMediaErrorCode::InputNotFound)
            }
            std::io::ErrorKind::PermissionDenied => {
                LocalMediaError::new(LocalMediaErrorCode::InputNotFound)
            }
            _ => LocalMediaError::new(LocalMediaErrorCode::UnsupportedMediaType),
        })?;
        if link_metadata.file_type().is_symlink() {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::UnsupportedMediaType,
            ));
        }
        // `is_file` is true only for regular files: directories, FIFOs, sockets, and device nodes
        // all fail here, which is what keeps staging from blocking on a pipe.
        if !link_metadata.is_file() {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::UnsupportedMediaType,
            ));
        }
        let byte_length = link_metadata.len();
        if byte_length == 0 {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::UnsupportedMediaType,
            ));
        }
        // The size ceiling is applied by `admit_content`, after this read. That ordering is safe
        // because the read itself is capped at `HEADER_SCAN_BYTES` regardless of how large the file
        // claims to be, so an oversized file costs one bounded read and is refused before any copy.
        let mut handle = fs::File::open(source).map_err(|_| Self::storage_error())?;
        let mut prefix = vec![0u8; HEADER_SCAN_BYTES.min(byte_length as usize)];
        let read = handle
            .read(&mut prefix)
            .map_err(|_| Self::storage_error())?;
        prefix.truncate(read);
        Ok((prefix, byte_length))
    }

    /// Content checks shared by the path and the bytes entry points.
    fn admit_content(
        &self,
        prefix: &[u8],
        byte_length: u64,
    ) -> Result<SniffedFormat, LocalMediaError> {
        if byte_length > self.limits.max_input_bytes {
            return Err(LocalMediaError::new(LocalMediaErrorCode::InputTooLarge)
                .with_number("limit", self.limits.max_input_bytes as i64)
                .with_number("actual", byte_length as i64));
        }
        let Some(format) = sniff_media(prefix) else {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::UnsupportedMediaType,
            ));
        };
        if format.media_type() == OcrMediaType::Image {
            // No dimensions means no enforceable ceiling, so the file is refused rather than
            // admitted with the check skipped.
            let Some(dimensions) = image_dimensions(format, prefix) else {
                return Err(LocalMediaError::new(
                    LocalMediaErrorCode::UnsupportedMediaType,
                ));
            };
            if exceeds_pixel_limit(dimensions, self.limits.max_decoded_pixels) {
                return Err(
                    LocalMediaError::new(LocalMediaErrorCode::ImagePixelLimitExceeded)
                        .with_number("limit", self.limits.max_decoded_pixels as i64),
                );
            }
        }
        Ok(format)
    }

    fn record_staged(
        &self,
        format: SniffedFormat,
        display_name: String,
        byte_length: u64,
        write: impl FnOnce(&Path) -> Result<(), LocalMediaError>,
    ) -> Result<StagedOcrSource, LocalMediaError> {
        let staged_input_id = StagedInputId::new(self.ids.next(StagedInputId::PREFIX));
        let directory = self.owned_directory(STAGING_DIR, staged_input_id.as_str())?;
        Self::ensure_directory(&directory)?;
        let target = directory.join(format!("source.{}", format.extension()));
        write(&target)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&target, fs::Permissions::from_mode(0o600));
        }

        let source = StagedOcrSource {
            staged_input_id: staged_input_id.clone(),
            display_name,
            media_type: format.media_type(),
            byte_length,
        };
        if let Ok(mut staged) = self.staged.lock() {
            staged.insert(
                staged_input_id.as_str().to_string(),
                StagedInputRecord::new(staged_input_id, source.clone(), self.clock.now_ms()),
            );
        }
        Ok(source)
    }

    fn remove_directory(path: &Path) {
        // A symlinked tier directory would make `remove_dir_all` delete somewhere else, so the
        // link case is unlinked rather than recursed into.
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let _ = fs::remove_file(path);
            }
            Ok(_) => {
                let _ = fs::remove_dir_all(path);
            }
            Err(_) => {}
        }
    }

    fn sweep_tier(&self, tier: &str, older_than_ms: u64, budget: &mut usize) -> usize {
        let Ok(entries) = fs::read_dir(self.root.join(tier)) else {
            return 0;
        };
        let cutoff = SystemTime::now().checked_sub(Duration::from_millis(older_than_ms));
        let mut removed = 0usize;
        for entry in entries.flatten() {
            if *budget == 0 {
                break;
            }
            *budget -= 1;
            let path = entry.path();
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.file_type().is_symlink() {
                // A reparse point inside our own root is not something this code created.
                let _ = fs::remove_file(&path);
                removed += 1;
                continue;
            }
            let is_stale = match (cutoff, metadata.modified()) {
                (Some(cutoff), Ok(modified)) => modified <= cutoff,
                // No usable timestamp: leave it. A sweep that guesses would delete live media.
                _ => older_than_ms == 0,
            };
            if is_stale {
                Self::remove_directory(&path);
                removed += 1;
            }
        }
        removed
    }
}

impl MediaTempStore for FilesystemMediaTempStore {
    fn stage_ocr_source(&self, source: &Path) -> Result<StagedOcrSource, LocalMediaError> {
        let (prefix, byte_length) = Self::read_admitted_prefix(source)?;
        let format = self.admit_content(&prefix, byte_length)?;
        let display_name = sanitize_display_name(&source.to_string_lossy());
        let source = source.to_path_buf();
        self.record_staged(format, display_name, byte_length, move |target| {
            // Copy rather than move: the user's file is theirs, and a move would make a cancelled
            // OCR destructive.
            fs::copy(&source, target)
                .map(|_| ())
                .map_err(|_| Self::storage_error())
        })
    }

    fn stage_bytes(
        &self,
        bytes: &[u8],
        display_name: &str,
    ) -> Result<StagedOcrSource, LocalMediaError> {
        let byte_length = bytes.len() as u64;
        let format = self.admit_content(bytes, byte_length)?;
        let display_name = sanitize_display_name(display_name);
        self.record_staged(format, display_name, byte_length, |target| {
            Self::write_private_file(target, bytes)
        })
    }

    fn claim(&self, staged_input_id: &StagedInputId) -> Result<ClaimedInput, LocalMediaError> {
        let now_ms = self.clock.now_ms();
        let Ok(mut staged) = self.staged.lock() else {
            return Err(LocalMediaError::new(LocalMediaErrorCode::InputNotFound));
        };
        let Some(record) = staged.get_mut(staged_input_id.as_str()) else {
            return Err(LocalMediaError::new(LocalMediaErrorCode::InputNotFound));
        };
        record.claim(now_ms).map_err(LocalMediaError::new)?;
        let source = record.source().clone();
        drop(staged);

        let directory = self.owned_directory(STAGING_DIR, staged_input_id.as_str())?;
        let Ok(entries) = fs::read_dir(&directory) else {
            return Err(LocalMediaError::new(LocalMediaErrorCode::InputNotFound));
        };
        let Some(path) = entries
            .flatten()
            .map(|entry| entry.path())
            .find(|path| path.is_file())
        else {
            return Err(LocalMediaError::new(LocalMediaErrorCode::InputNotFound));
        };
        Ok(ClaimedInput {
            staged_input_id: staged_input_id.clone(),
            source,
            path,
        })
    }

    fn authorize_recording_wav(
        &self,
        recording_id: &RecordingId,
    ) -> Result<PathBuf, LocalMediaError> {
        let directory = self.owned_directory(RECORDINGS_DIR, recording_id.as_str())?;
        Self::ensure_directory(&directory)?;
        Ok(directory.join(RECORDING_FILE))
    }

    fn cleanup_recording(&self, recording_id: &RecordingId) {
        if let Ok(directory) = self.owned_directory(RECORDINGS_DIR, recording_id.as_str()) {
            Self::remove_directory(&directory);
        }
    }

    fn authorize_output_wav(&self, operation_id: &str) -> Result<PathBuf, LocalMediaError> {
        let directory = self.owned_directory(OPERATIONS_DIR, operation_id)?;
        Self::ensure_directory(&directory)?;
        Ok(directory.join(OUTPUT_FILE))
    }

    fn verify_output_wav(
        &self,
        operation_id: &str,
        candidate: &Path,
    ) -> Result<u64, LocalMediaError> {
        let protocol_error = || LocalMediaError::new(LocalMediaErrorCode::WorkerProtocolError);
        let expected = self
            .owned_directory(OPERATIONS_DIR, operation_id)
            .map_err(|_| protocol_error())?
            .join(OUTPUT_FILE);
        if candidate != expected {
            return Err(protocol_error());
        }
        let metadata = fs::symlink_metadata(&expected).map_err(|_| protocol_error())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(protocol_error());
        }
        let byte_length = metadata.len();
        if byte_length == 0 || byte_length > MAX_OUTPUT_WAV_BYTES {
            return Err(protocol_error());
        }
        let mut header = [0u8; 12];
        let mut handle = fs::File::open(&expected).map_err(|_| protocol_error())?;
        handle
            .read_exact(&mut header)
            .map_err(|_| protocol_error())?;
        if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
            return Err(protocol_error());
        }
        Ok(byte_length)
    }

    fn cleanup_operation(&self, operation_id: &str) {
        if let Ok(directory) = self.owned_directory(OPERATIONS_DIR, operation_id) {
            Self::remove_directory(&directory);
        }
    }

    fn cleanup_staged(&self, staged_input_id: &StagedInputId) {
        if let Ok(mut staged) = self.staged.lock() {
            staged.remove(staged_input_id.as_str());
        }
        if let Ok(directory) = self.owned_directory(STAGING_DIR, staged_input_id.as_str()) {
            Self::remove_directory(&directory);
        }
    }

    fn sweep_stale(&self, older_than_ms: u64) -> usize {
        let mut budget = MAX_SWEEP_ENTRIES;
        let mut removed = 0;
        for tier in [STAGING_DIR, RECORDINGS_DIR, OPERATIONS_DIR] {
            removed += self.sweep_tier(tier, older_than_ms, &mut budget);
        }
        if let Ok(mut staged) = self.staged.lock() {
            let now_ms = self.clock.now_ms();
            staged.retain(|_, record| !record.is_expired(now_ms));
        }
        removed
    }
}

#[cfg(test)]
#[path = "temp_store_tests.rs"]
mod tests;
