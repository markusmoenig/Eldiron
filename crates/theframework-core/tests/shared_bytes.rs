use theframework_core::SharedBytes;

#[test]
fn frozen_assets_share_pixels_until_an_author_edits_and_keep_vec_wire_bytes() {
    for source in [
        vec![],
        vec![0, 255, 1, 128],
        (0..=255).cycle().take(16384).collect(),
    ] {
        let mut authored = SharedBytes::from(source.clone());
        let mut frozen = authored.clone();
        assert_eq!(authored.as_ptr(), frozen.as_ptr());
        let json = serde_json::to_vec(&source).unwrap();
        let binary = bincode::serialize(&source).unwrap();
        assert_eq!(serde_json::to_vec(&authored).unwrap(), json);
        assert_eq!(bincode::serialize(&authored).unwrap(), binary);
        assert_eq!(
            &*serde_json::from_slice::<SharedBytes>(&json).unwrap(),
            &source
        );
        assert_eq!(
            &*bincode::deserialize::<SharedBytes>(&binary).unwrap(),
            &source
        );
        authored.push(77);
        assert_eq!(&*frozen, &source, "author edits cannot mutate a frozen job");
        assert_ne!(authored.as_ptr(), frozen.as_ptr());
        frozen.resize(2, 99);
        assert_eq!(authored.last(), Some(&77));
        assert_eq!(Vec::<u8>::from(authored).len(), source.len() + 1);
    }
}
