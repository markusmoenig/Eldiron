use character_generator::*;
const SOURCE: &str = include_str!("../examples/villager.toml");
#[test]
fn recipes_texture_the_mesh_and_reject_unresolved_or_unsupported_surfaces() {
    let catalog = Catalog::parse(SOURCE).unwrap();
    let a = catalog.generate("villager").unwrap();
    let shirt = a
        .materials
        .iter()
        .find(|m| m.name == "linen_shirt")
        .unwrap();
    let texture = shirt.texture.as_ref().unwrap();
    assert_eq!(texture.rgba.len(), 64 * 64 * 4);
    assert_eq!(texture.orm.len(), 64 * 64 * 4);
    assert!(
        texture
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p != &texture.rgba[..4])
    );
    assert!(
        texture
            .orm
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [255, 235, 0, 255])
    );
    let different_seed = Catalog::parse(&SOURCE.replace("seed = 419", "seed = 420"))
        .unwrap()
        .generate("villager")
        .unwrap();
    let changed_texture = different_seed
        .materials
        .iter()
        .find(|m| m.name == "linen_shirt")
        .unwrap()
        .texture
        .as_ref()
        .unwrap();
    assert_ne!(texture.rgba, changed_texture.rgba);
    let explicit_palette = SOURCE.replace("size = 64", "size = 64\npalette = [\"#ffffff\"]");
    let uniform = Catalog::parse(&explicit_palette)
        .unwrap()
        .generate("villager")
        .unwrap();
    assert!(
        uniform
            .materials
            .iter()
            .find(|m| m.name == "linen_shirt")
            .unwrap()
            .texture
            .as_ref()
            .unwrap()
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [255, 255, 255, 255])
    );
    let copied = catalog.generate("villager").unwrap();
    assert_eq!(
        serde_json::to_vec(&a).unwrap(),
        serde_json::to_vec(&copied).unwrap()
    );
    for (before, after) in [
        ("material = \"linen\"", "material = \"missing\""),
        ("size = 64", "size = 4096"),
        ("tiling = [2.0, 2.0]", "tiling = [-1.0, 2.0]"),
    ] {
        assert!(Catalog::parse(&SOURCE.replace(before, after)).is_err());
    }
    for (before, after) in [
        ("opacity = 1.0", "opacity = 0.5"),
        ("emission = 0.0", "emission = 1.0"),
    ] {
        assert!(
            Catalog::parse(&SOURCE.replace(before, after))
                .unwrap()
                .generate("villager")
                .is_err()
        );
    }
}
#[test]
fn ruleset_adapter_ignores_gameplay_fields_and_detects_duplicate_ids() {
    let source = include_str!("../examples/ruleset-overlay.toml");
    let root: toml::Table = toml::from_str(source).unwrap();
    let catalog = ruleset::catalog_from_table(&root).unwrap();
    let a = catalog.generate("villager").unwrap();
    assert!(
        a.primitives
            .iter()
            .any(|p| p.name == "training_sword_blade")
    );
    let duplicate = format!(
        "{source}\n[items.other.training_sword.appearance]\ngenerator = \"sword\"\ncolor = \"#ffffff\"\nattachment = \"main_hand\"\n"
    );
    assert!(ruleset::catalog_from_table(&toml::from_str(&duplicate).unwrap()).is_err());
    let miss = source.replace(
        "equipment = [\"training_sword\"]",
        "equipment = [\"missing\"]",
    );
    assert!(ruleset::catalog_from_table(&toml::from_str(&miss).unwrap()).is_err());
}
