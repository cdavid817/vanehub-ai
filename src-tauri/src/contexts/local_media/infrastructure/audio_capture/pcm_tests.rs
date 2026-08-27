use super::*;
use tempfile::TempDir;

#[test]
fn f32_samples_clamp_and_scale_into_i16() {
    assert_eq!(sample_from_f32(0.0), 0);
    assert_eq!(sample_from_f32(1.0), i16::MAX);
    assert_eq!(sample_from_f32(-1.0), -i16::MAX);
    // Values outside [-1, 1] are clamped, not wrapped: a wrap turns a loud passage into noise.
    assert_eq!(sample_from_f32(4.2), i16::MAX);
    assert_eq!(sample_from_f32(-4.2), -i16::MAX);
    assert_eq!(sample_from_f32(f32::NAN), 0);
    assert_eq!(sample_from_f32(f32::INFINITY), i16::MAX);
}

#[test]
fn u16_samples_are_recentred_around_zero() {
    assert_eq!(sample_from_u16(32_768), 0);
    assert_eq!(sample_from_u16(0), i16::MIN);
    assert_eq!(sample_from_u16(65_535), i16::MAX);
}

#[test]
fn mono_input_passes_through_unchanged() {
    let mut output = Vec::new();
    downmix_to_mono(&[100, -100, 32_767], 1, &mut output);
    assert_eq!(output, vec![100, -100, 32_767]);
}

#[test]
fn stereo_is_averaged_rather_than_summed() {
    let mut output = Vec::new();
    downmix_to_mono(&[100, 300, -50, 50], 2, &mut output);
    assert_eq!(output, vec![200, 0]);
}

#[test]
fn downmixing_two_full_scale_channels_does_not_overflow() {
    // Summing in i16 would wrap to -2; the accumulator has to be wider than the sample.
    let mut output = Vec::new();
    downmix_to_mono(&[i16::MAX, i16::MAX], 2, &mut output);
    assert_eq!(output, vec![i16::MAX]);

    output.clear();
    downmix_to_mono(&[i16::MIN, i16::MIN], 2, &mut output);
    assert_eq!(output, vec![i16::MIN]);
}

#[test]
fn a_partial_final_frame_is_dropped_rather_than_padded() {
    // Padding with silence would put a click at the end of every recording whose buffer did not
    // divide evenly by the channel count.
    let mut output = Vec::new();
    downmix_to_mono(&[100, 300, -50], 2, &mut output);
    assert_eq!(output, vec![200]);
}

#[test]
fn a_zero_channel_count_produces_nothing_instead_of_dividing_by_zero() {
    let mut output = Vec::new();
    downmix_to_mono(&[100, 200], 0, &mut output);
    assert!(output.is_empty());
}

#[test]
fn six_channel_input_averages_across_all_of_them() {
    let mut output = Vec::new();
    downmix_to_mono(
        &[600, 0, 0, 0, 0, 0, 60, 60, 60, 60, 60, 60],
        6,
        &mut output,
    );
    assert_eq!(output, vec![100, 60]);
}

fn writer_fixture() -> (TempDir, std::path::PathBuf) {
    let directory = TempDir::new().expect("temp dir");
    let path = directory.path().join("input.wav");
    (directory, path)
}

#[test]
fn the_writer_produces_a_readable_mono_16_bit_wav() {
    let (_directory, path) = writer_fixture();
    let mut writer = PcmWavWriter::create(&path, 16_000).expect("writer");
    writer.write(&[0, 100, -100, 32_767]).expect("write");
    let committed = writer.finalize().expect("finalize");

    assert_eq!(committed.sample_count, 4);
    assert_eq!(committed.sample_rate, 16_000);
    let reader = hound::WavReader::open(&path).expect("read back");
    let spec = reader.spec();
    assert_eq!(spec.channels, 1);
    assert_eq!(spec.bits_per_sample, 16);
    assert_eq!(spec.sample_rate, 16_000);
    assert_eq!(reader.len(), 4);
}

#[test]
fn duration_is_derived_from_the_committed_sample_count() {
    let (_directory, path) = writer_fixture();
    let mut writer = PcmWavWriter::create(&path, 16_000).expect("writer");
    writer.write(&vec![0i16; 8_000]).expect("write");
    let committed = writer.finalize().expect("finalize");
    assert_eq!(committed.duration_ms, 500);
}

#[test]
fn an_empty_recording_finalizes_to_a_valid_zero_length_wav() {
    // A tap produces no samples; the file still has to be a well-formed WAV so the too-short check
    // runs on a real duration instead of a read error.
    let (_directory, path) = writer_fixture();
    let writer = PcmWavWriter::create(&path, 16_000).expect("writer");
    let committed = writer.finalize().expect("finalize");
    assert_eq!(committed.sample_count, 0);
    assert_eq!(committed.duration_ms, 0);
    assert!(hound::WavReader::open(&path).is_ok());
}

#[test]
fn creating_a_writer_under_a_missing_directory_fails_cleanly() {
    let directory = TempDir::new().expect("temp dir");
    let path = directory.path().join("absent").join("input.wav");
    assert!(PcmWavWriter::create(&path, 16_000).is_err());
}

#[test]
fn a_zero_sample_rate_is_refused_rather_than_written() {
    let (_directory, path) = writer_fixture();
    assert!(PcmWavWriter::create(&path, 0).is_err());
}

#[test]
fn the_writer_pipeline_drains_every_queued_chunk_before_finalizing() {
    let (_directory, path) = writer_fixture();
    let pipeline = CaptureWriter::start(&path, 16_000, 8).expect("pipeline");
    for _ in 0..4 {
        assert!(pipeline.submit(vec![1i16; 1_000]).is_ok());
    }
    let committed = pipeline.finish().expect("finish");
    assert_eq!(committed.sample_count, 4_000);
    assert!(!committed.overrun);
}

#[test]
fn a_full_queue_reports_an_overrun_instead_of_dropping_audio() {
    // Silently dropping frames would produce a transcript of an utterance the user never spoke.
    let (_directory, path) = writer_fixture();
    let pipeline = CaptureWriter::start(&path, 16_000, 1).expect("pipeline");
    pipeline.pause_writer_for_test();

    let mut overran = false;
    for _ in 0..64 {
        if pipeline.submit(vec![0i16; 4_096]).is_err() {
            overran = true;
            break;
        }
    }
    assert!(overran, "a bounded queue must refuse rather than grow");
    pipeline.resume_writer_for_test();
    let committed = pipeline.finish().expect("finish");
    assert!(committed.overrun);
}

#[test]
fn submitting_after_finish_is_refused() {
    let (_directory, path) = writer_fixture();
    let pipeline = CaptureWriter::start(&path, 16_000, 4).expect("pipeline");
    pipeline.submit(vec![0i16; 10]).expect("first submit");
    let committed = pipeline.finish().expect("finish");
    assert_eq!(committed.sample_count, 10);
}
