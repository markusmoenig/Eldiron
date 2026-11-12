use super::*;

#[test]
fn audio_ogg_streaming_decode_downmixes_and_enforces_pcm_limits() {
    // Generated without a device: ffmpeg -f lavfi -i
    // sine=frequency=440:sample_rate=8000:duration=0.05 -ac 2
    // -c:a vorbis -strict experimental stereo-tone.ogg
    let bytes = include_bytes!("fixtures/stereo-tone.ogg");
    let clip = decode_clip_limited(bytes, 2048).unwrap();
    assert_eq!(clip.sample_rate, 8000);
    assert!((400..=2048).contains(&clip.samples.len()));
    assert!(clip.samples.iter().all(|s| s.is_finite()));
    let rms = (clip.samples.iter().map(|s| s * s).sum::<f32>() / clip.samples.len() as f32).sqrt();
    assert!(rms > 0.01, "the actual codec must produce non-silent PCM");
    assert!(matches!(
        decode_clip_limited(bytes, 8),
        Err(AudioError::PcmBudget)
    ));
}
fn wav(channels: u16, samples: &[i16]) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(
        &mut bytes,
        hound::WavSpec {
            channels,
            sample_rate: 8000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for sample in samples {
        writer.write_sample(*sample).unwrap();
    }
    writer.finalize().unwrap();
    bytes.into_inner()
}
fn clip(samples: &[f32]) -> DecodedClip {
    DecodedClip {
        sample_rate: 8000,
        samples: Arc::from(samples),
    }
}
#[test]
fn audio_pcm_decoder_downmixes_stereo_and_refuses_to_grow_past_the_sample_budget() {
    let bytes = wav(2, &[16384, 0, -16384, 0]);
    let decoded = decode_clip_limited(&bytes, 2).unwrap();
    assert_eq!(decoded.sample_rate, 8000);
    assert_eq!(decoded.samples.as_ref(), &[0.25, -0.25]);
    assert!(
        decode_clip_limited(&bytes, 1)
            .unwrap_err()
            .to_string()
            .contains("budget")
    );
    assert!(decode_clip_limited(&bytes, 0).is_err());
    assert_eq!(
        decode_clip(&bytes).unwrap().samples.as_ref(),
        decoded.samples.as_ref()
    );
}
#[test]
fn audio_resident_budget_counts_shared_pcm_once_and_keeps_active_data_pinned_after_eviction() {
    let mut mixer = MixerState::new(1.);
    mixer.insert_clip("music".into(), clip(&[0.2, 0.4, 0.6, 0.8]));
    assert_eq!(mixer.pcm_bytes_without_cache(""), 16);
    assert_eq!(mixer.pcm_bytes_without_cache("music"), 0);
    assert!(mixer.play_clip(8000, "music", "music", 1., true));
    assert!(mixer.play_clip(8000, "music", "ambience", 1., true));
    assert_eq!(mixer.pcm_bytes_without_cache("music"), 16);
    mixer.clips.remove("music");
    assert_eq!(mixer.pcm_bytes_without_cache(""), 16);
    mixer.clear_bus("music");
    assert_eq!(mixer.pcm_bytes_without_cache(""), 16);
    mixer.clear_all_buses();
    assert_eq!(mixer.pcm_bytes_without_cache(""), 0);
}
#[test]
fn audio_voice_limit_bus_gain_and_sample_rate_conversion_apply_to_real_mixer_output() {
    let mut mixer = MixerState::new(0.5);
    mixer.insert_clip("one".into(), clip(&[0., 1., 0.]));
    mixer.set_bus_volume("sfx", 0.5);
    assert!(mixer.play_clip_bounded(16000, "one", "sfx", 0.5, false, 1));
    assert!(!mixer.play_clip_bounded(16000, "one", "sfx", 0.5, false, 1));
    let actual = (0..6).map(|_| mixer.mix_next_sample()).collect::<Vec<_>>();
    assert_eq!(actual, vec![0., 0.0625, 0.125, 0.0625, 0., 0.]);
    assert!(mixer.clip_voices.is_empty());
    assert!(mixer.play_clip_bounded(16000, "one", "sfx", 0.5, false, 1));
}
#[test]
fn audio_loop_boundaries_do_not_insert_a_silent_sample_or_break_interpolation() {
    let mut mixer = MixerState::new(1.);
    mixer.insert_clip("loop".into(), clip(&[1., -1.]));
    mixer.play_clip(8000, "loop", "music", 1., true);
    assert_eq!(
        (0..6).map(|_| mixer.mix_next_sample()).collect::<Vec<_>>(),
        vec![1., -1., 1., -1., 1., -1.]
    );
    mixer.clear_all_buses();
    mixer.play_clip(16000, "loop", "music", 1., true);
    assert_eq!(
        (0..8).map(|_| mixer.mix_next_sample()).collect::<Vec<_>>(),
        vec![1., 0., -1., 0., 1., 0., -1., 0.]
    );
}
#[test]
fn generated_audio_fx_use_the_same_bounded_decoder_and_do_not_require_a_device() {
    let bytes = synthesize_audio_fx_wav("[sfx.hit]\nwave='sine'\nduration=0.02\ngain=0.2\n", "hit")
        .unwrap();
    let clip = decode_clip_limited(&bytes, 44100).unwrap();
    assert!(!clip.samples.is_empty());
    assert!(clip.samples.iter().all(|s| s.is_finite()));
    assert!(decode_clip_limited(&bytes, 1).is_err());
    assert!(matches!(
        synthesize_audio_fx_wav_bounded(
            "[sfx.huge]\nduration=1e20\nrepeat=9223372036854775807\n",
            "huge",
            1024
        ),
        Err(AudioError::PcmBudget)
    ));
    assert!(matches!(
        synthesize_audio_fx_wav_bounded(
            "[sfx.huge]\nduration=0.01\nrepeat=1\nrepeat_gap=1e30\n",
            "huge",
            4096
        ),
        Err(AudioError::PcmBudget)
    ));
    assert_eq!(
        synthesize_audio_fx_wav_bounded(
            "[sfx.hit]\nwave='sine'\nduration=0.02\ngain=0.2\n",
            "hit",
            44100 * 4
        )
        .unwrap(),
        bytes
    );
}
#[test]
fn audio_long_music_positions_advance_after_the_f32_integer_precision_boundary() {
    let base = 1usize << 24;
    let mut samples = vec![0.; base + 3];
    samples[base..].copy_from_slice(&[0.25, 0.5, 0.75]);
    let mut mixer = MixerState::new(1.);
    mixer.insert_clip(
        "long".into(),
        DecodedClip {
            sample_rate: 44100,
            samples: samples.into(),
        },
    );
    mixer.play_clip(44100, "long", "music", 1., false);
    mixer.clip_voices[0].pos = base as _;
    assert_eq!(
        (0..3).map(|_| mixer.mix_next_sample()).collect::<Vec<_>>(),
        vec![0.25, 0.5, 0.75]
    );
}
