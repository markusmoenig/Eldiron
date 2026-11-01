use denrim_materials::metadata::{Kernel, catalog_json};
use std::collections::HashSet;

#[test]
fn every_tile_has_a_unique_stable_help_identity() {
    let mut ids = HashSet::new();
    let mut keys = HashSet::new();
    let catalog = catalog_json();
    for kernel in Kernel::ALL {
        let id = kernel.source_id();
        assert!(
            !id.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        );
        assert!(ids.insert(id), "Duplicate source identity: {id}");
        assert!(keys.insert(kernel.help_key()));
        assert!(catalog.contains(&format!("\"source_id\":\"{id}\"")));
        assert!(catalog.contains(&format!("\"help_key\":\"tile.{id}\"")));
    }
    assert_eq!(Kernel::Perlin.source_id(), "perlin");
    assert_eq!(
        Kernel::find("Organic Noise").unwrap().help_key(),
        "tile.organic_noise"
    );
    assert_eq!(
        Kernel::find("Rounded Rectangle").unwrap().source_id(),
        "rounded_rectangle"
    );
}
