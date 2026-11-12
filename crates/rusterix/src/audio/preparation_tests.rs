use super::*;
fn generated(id: u64, source: &str) -> PreparationJob {
    PreparationJob {
        id,
        source: AudioSource::Generated(AudioEffect::parse(source, "tone").unwrap()),
        pcm_bytes: 1_000_000,
        generated_pcm_bytes: 1_000_000,
        cancelled: Arc::new(AtomicBool::new(false)),
    }
}
#[test]
fn prepared_generated_pcm_matches_reference_quantization_and_rejects_oversized_or_cancelled_work() {
    let source =
        "[sfx.tone]\nwave='noise'\nduration=0.03\nrepeat=2\nrepeat_gap=0.01\nlowpass=0.5\n";
    let reference =
        PreparedClip::decode(&synthesize_audio_fx_wav(source, "tone").unwrap(), 1_000_000).unwrap();
    let prepared = generated(7, source).run().clip.unwrap();
    assert_eq!(prepared.sample_rate(), reference.sample_rate());
    assert_eq!(prepared.samples(), reference.samples());
    assert_eq!(prepared.pcm_bytes(), reference.samples().len() * 4);
    let mut bounded = generated(8, source);
    bounded.pcm_bytes = 64;
    assert!(matches!(bounded.run().clip, Err(AudioError::PcmBudget)));
    let cancelled = generated(9, "[sfx.tone]\nduration=1e30\n");
    cancelled.cancelled.store(true, Ordering::Relaxed);
    assert!(matches!(cancelled.run().clip, Err(AudioError::Cancelled)));
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_preparation_admits_one_job_and_reuses_worker_for_actual_generated_and_ogg_sources() {
    let mut worker = ThreadPreparation::default();
    worker
        .start(generated(1, "[sfx.tone]\nduration=0.02\n"))
        .unwrap();
    assert!(!worker.is_idle());
    assert!(worker.start(generated(99, "[sfx.tone]\n")).is_err());
    fn wait(worker: &mut ThreadPreparation) -> PreparationResult {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(result) = worker.poll() {
                return result;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "CPU worker did not publish completion"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    let result = wait(&mut worker);
    assert_eq!(result.id, 1);
    assert_eq!(result.clip.unwrap().samples().len(), 882);
    assert!(worker.is_idle());
    worker
        .start(PreparationJob {
            source: AudioSource::Encoded(
                include_bytes!("fixtures/stereo-tone.ogg").as_slice().into(),
            ),
            ..generated(2, "[sfx.tone]\n")
        })
        .unwrap();
    let result = wait(&mut worker);
    assert_eq!(result.id, 2);
    let ogg = result.clip.unwrap();
    assert_eq!(ogg.sample_rate(), 8000);
    assert!(ogg.samples().len() >= 400);
    let cancelled = generated(3, "[sfx.tone]\nduration=1e30\n");
    cancelled.cancelled.store(true, Ordering::Relaxed);
    worker.start(cancelled).unwrap();
    assert!(matches!(wait(&mut worker).clip, Err(AudioError::Cancelled)));
}
