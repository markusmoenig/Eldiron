use crate::prelude::*;
use denrim_materials::geometry::{Document, Operation, WEARER_SOURCE};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Owner {
    Character(Uuid),
    Item(Uuid),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Geometry,
    Material,
    Icon,
}
#[derive(Clone, Debug)]
pub struct Asset {
    pub id: Uuid,
    pub label: String,
    pub kind: Kind,
    pub active_state: Option<bool>,
}
impl Owner {
    pub fn id(self) -> Uuid {
        match self {
            Self::Character(id) | Self::Item(id) => id,
        }
    }
    pub fn name(self, p: &Project) -> Option<&str> {
        match self {
            Self::Character(id) => p.characters.get(&id).map(|c| c.name.as_str()),
            Self::Item(id) => p.items.get(&id).map(|i| i.name.as_str()),
        }
    }
    pub fn data(self, p: &Project) -> Option<&str> {
        match self {
            Self::Character(id) => p.characters.get(&id).map(|c| c.data.as_str()),
            Self::Item(id) => p.items.get(&id).map(|i| i.data.as_str()),
        }
    }
    pub fn set_data(self, p: &mut Project, text: String) -> Result<(), String> {
        match self {
            Self::Character(id) => p.characters.get_mut(&id).map(|c| c.data = text),
            Self::Item(id) => p.items.get_mut(&id).map(|i| i.data = text),
        }
        .ok_or("Appearance owner was removed".into())
    }
}
pub fn attribute(data: &str, key: &str) -> Option<String> {
    let t = data.parse::<toml::Table>().ok()?;
    t.get("attributes")
        .and_then(|t| t.get(key))
        .or_else(|| t.get(key))?
        .as_str()
        .map(str::to_owned)
}
pub fn with_attribute(data: &str, key: &str, value: &str) -> Result<String, String> {
    let mut t = data
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| e.to_string())?;
    t.remove(key);
    if t.get("attributes").is_none() {
        t["attributes"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    if !t["attributes"].is_table_like() {
        return Err("Attributes must be a table".into());
    }
    t["attributes"][key] = toml_edit::value(value);
    Ok(t.to_string())
}
pub fn bind_character_model(data: &str, source: &str) -> Result<String, String> {
    let data = with_attribute(data, "avatar", source)?;
    let mut doc = data
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| e.to_string())?;
    for key in ["avatar_id", "tile_id", "source"] {
        doc.remove(key);
        if let Some(attrs) = doc["attributes"].as_table_like_mut() {
            attrs.remove(key);
        }
    }
    Ok(doc.to_string())
}
pub fn geometry_id(source: &str) -> Option<Uuid> {
    shared::appearance_assets::asset_id(source)
        .or_else(|_| shared::appearance_assets::asset_id(&format!("ruleset:geometry/{source}")))
        .ok()
}
pub fn body_source(owner: Owner, p: &Project) -> String {
    let data = owner.data(p).unwrap_or("");
    if matches!(owner, Owner::Character(_)) {
        let mut entity = rusterix::Entity::default();
        rusterix::server::data::apply_entity_data(&mut entity, data);
        let mut assets = rusterix::Assets::new();
        assets.rules = p.rules_source().unwrap_or_default();
        let entity = rusterix::server::data::character_for_display(&entity, &assets);
        entity
            .attributes
            .get_id("avatar_id")
            .map(|id| id.to_string())
            .or_else(|| entity.attributes.get_str("avatar").map(str::to_owned))
            .or_else(|| attribute(data, "source"))
            .or_else(|| attribute(data, "tile_id"))
            .unwrap_or_else(|| "humanoid".into())
    } else {
        attribute(data, "appearance_geometry").unwrap_or_default()
    }
}
fn label(source: &str, p: &Project) -> String {
    let id = source
        .strip_prefix("ruleset:material/")
        .map(shared::rulesets::material_asset_id)
        .or_else(|| Uuid::parse_str(source).ok());
    id.and_then(|id| p.tiles.get(&id))
        .map(|t| t.alias.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            source
                .trim_start_matches("ruleset:material/")
                .replace(['-', '_'], " ")
        })
}
pub fn assets(owner: Owner, p: &Project) -> Vec<Asset> {
    fn visit(
        doc: &Document,
        p: &Project,
        materials: &mut BTreeMap<Uuid, String>,
        seen: &mut BTreeSet<Uuid>,
        depth: usize,
    ) {
        if depth > 6 {
            return;
        }
        for tile in doc.rows.iter().flat_map(|r| &r.tiles) {
            match &tile.op {
                Operation::Texture { source, parts, .. } => {
                    if let Some(id) = source
                        .strip_prefix("ruleset:material/")
                        .map(shared::rulesets::material_asset_id)
                        .or_else(|| Uuid::parse_str(source).ok())
                    {
                        let name = label(source, p);
                        materials.entry(id).or_insert_with(|| {
                            if parts.is_empty() {
                                name
                            } else {
                                format!("{} · {name}", parts.join(", "))
                            }
                        });
                    }
                }
                Operation::Group { document, .. } => visit(document, p, materials, seen, depth + 1),
                Operation::Asset { source, document } if source != WEARER_SOURCE => {
                    let current = geometry_id(source)
                        .filter(|id| seen.insert(*id))
                        .and_then(|id| p.procedural_appearances.get(&id))
                        .and_then(|s| serde_json::from_str::<Document>(s).ok());
                    visit(
                        current.as_ref().unwrap_or(document),
                        p,
                        materials,
                        seen,
                        depth + 1,
                    );
                }
                _ => {}
            }
        }
    }
    let mut result = vec![];
    let mut materials = BTreeMap::new();
    let data = owner.data(p).unwrap_or("");
    let mut geometries = vec![(
        "Model / rig".to_string(),
        geometry_id(&body_source(owner, p)),
    )];
    if matches!(owner, Owner::Item(_)) {
        geometries.push((
            "Worn model".into(),
            shared::ruleset_materials::worn_geometry_id(data),
        ));
        geometries.push((
            "Model · inactive".into(),
            attribute(data, "appearance_geometry_off").and_then(|s| geometry_id(&s)),
        ));
    }
    let mut seen = BTreeSet::new();
    for (name, id) in geometries {
        if let Some(id) = id.filter(|id| p.procedural_appearances.contains_key(id)) {
            if result.iter().any(|a: &Asset| a.id == id) {
                continue;
            }
            result.push(Asset {
                id,
                label: name,
                kind: Kind::Geometry,
                active_state: None,
            });
            if let Some(doc) = p
                .procedural_appearances
                .get(&id)
                .and_then(|s| serde_json::from_str::<Document>(s).ok())
            {
                visit(&doc, p, &mut materials, &mut seen, 0);
            }
        }
    }
    if matches!(owner, Owner::Item(_)) {
        for source in shared::ruleset_materials::worn_material_sources(p, data) {
            if let Some(id) = source
                .strip_prefix("ruleset:material/")
                .map(shared::rulesets::material_asset_id)
                .or_else(|| Uuid::parse_str(&source).ok())
            {
                materials.entry(id).or_insert_with(|| label(&source, p));
            }
        }
    }
    result.extend(
        materials
            .into_iter()
            .filter(|(id, _)| p.procedural_materials.contains_key(id))
            .map(|(id, name)| Asset {
                id,
                label: format!("Material · {name}"),
                kind: Kind::Material,
                active_state: None,
            }),
    );
    if matches!(owner, Owner::Item(_)) {
        for (on, name) in [(true, "Icon · active"), (false, "Icon · inactive")] {
            if let Some(id) = shared::ruleset_materials::icon_id_for_state(data, on)
                .filter(|id| p.procedural_materials.contains_key(id))
            {
                result.push(Asset {
                    id,
                    label: name.into(),
                    kind: Kind::Icon,
                    active_state: Some(on),
                });
            }
        }
    }
    result
}
pub fn users(asset: &Asset, p: &Project) -> usize {
    p.characters
        .keys()
        .map(|id| Owner::Character(*id))
        .chain(p.items.keys().map(|id| Owner::Item(*id)))
        .filter(|owner| assets(*owner, p).iter().any(|a| a.id == asset.id))
        .count()
}
pub fn is_ruleset_asset(id: Uuid, p: &Project) -> bool {
    shared::ruleset_materials::definitions(&p.config)
        .iter()
        .any(|a| shared::rulesets::material_asset_id(a.id) == id)
        || shared::rulesets::bundled_appearance_assets_for_ruleset(
            shared::rulesets::OFFICIAL_RULESET_ID,
            shared::rulesets::OFFICIAL_RULESET_VERSION,
        )
        .iter()
        .any(|a| a.asset_id == id.to_string())
}
pub fn model_profile(source: &str, p: &Project) -> String {
    let defs = shared::rulesets::bundled_appearance_assets_for_ruleset(
        shared::rulesets::OFFICIAL_RULESET_ID,
        shared::rulesets::OFFICIAL_RULESET_VERSION,
    );
    if let Some(a) = defs.iter().find(|a| a.id == source || a.asset_id == source) {
        return a.id.into();
    }
    p.characters
        .values()
        .find_map(|c| {
            if attribute(&c.data, "avatar").as_deref() == Some(source) {
                attribute(&c.data, "appearance_profile")
            } else {
                None
            }
        })
        .unwrap_or_default()
}

pub fn character_models(p: &Project) -> Vec<String> {
    fn rigged(doc: &Document) -> bool {
        doc.rows.iter().flat_map(|r| &r.tiles).any(|t| match &t.op {
            Operation::Armature { .. } | Operation::Skeleton { .. } => true,
            Operation::Group { document, .. } | Operation::Asset { document, .. } => {
                rigged(document)
            }
            _ => false,
        })
    }
    let builtins = shared::rulesets::bundled_appearance_assets_for_ruleset(
        shared::rulesets::OFFICIAL_RULESET_ID,
        shared::rulesets::OFFICIAL_RULESET_VERSION,
    )
    .iter()
    .map(|a| a.asset_id.to_string())
    .collect::<BTreeSet<_>>();
    let mut models = vec!["humanoid".into(), "orc".into(), "skeleton".into()];
    models.extend(
        p.procedural_appearances
            .iter()
            .filter(|(id, _)| !builtins.contains(&id.to_string()))
            .filter_map(|(id, source)| {
                Document::load(source)
                    .ok()
                    .filter(rigged)
                    .map(|_| id.to_string())
            }),
    );
    models
}

/// Clone an owner's graph and material dependencies atomically. Equipment definitions remain shared.
pub fn make_variant(owner: Owner, p: &mut Project) -> Result<(), String> {
    fn material(
        id: Uuid,
        p: &mut Project,
        copies: &mut BTreeMap<Uuid, Uuid>,
    ) -> Result<Uuid, String> {
        if let Some(copy) = copies.get(&id) {
            return Ok(*copy);
        }
        let source = p
            .procedural_materials
            .get(&id)
            .cloned()
            .ok_or("Material is missing")?;
        let icon = shared::ruleset_materials::is_icon(&p.config, &id);
        let new = if icon {
            shared::ruleset_materials::new_icon_id()
        } else {
            Uuid::from_u128(
                (0x52554c454d415453u128 << 64) | (Uuid::new_v4().as_u128() & u64::MAX as u128),
            )
        };
        p.procedural_materials.insert(new, source);
        if let Some(tile) = p.tiles.get(&id).cloned() {
            let mut tile = tile;
            tile.id = new;
            tile.alias = format!("{} variant", tile.alias);
            p.tiles.insert(new, tile);
        }
        copies.insert(id, new);
        Ok(new)
    }
    fn document(
        doc: &mut Document,
        p: &mut Project,
        geo: &mut BTreeMap<Uuid, Uuid>,
        mats: &mut BTreeMap<Uuid, Uuid>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 6 {
            return Err("Appearance dependencies exceed six levels".into());
        }
        for tile in doc.rows.iter_mut().flat_map(|r| &mut r.tiles) {
            match &mut tile.op {
                Operation::Texture { source, bundle, .. } => {
                    if let Some(id) = source
                        .strip_prefix("ruleset:material/")
                        .map(shared::rulesets::material_asset_id)
                        .or_else(|| Uuid::parse_str(source).ok())
                        .filter(|id| p.procedural_materials.contains_key(id))
                    {
                        *source = material(id, p, mats)?.to_string();
                        *bundle = None;
                    }
                }
                Operation::Group {
                    document: inner, ..
                } => document(inner, p, geo, mats, depth + 1)?,
                Operation::Asset {
                    source,
                    document: inner,
                } if source != WEARER_SOURCE => {
                    if let Some(id) =
                        geometry_id(source).filter(|id| p.procedural_appearances.contains_key(id))
                    {
                        let new = geometry(id, p, geo, mats, depth + 1)?;
                        *source = new.to_string();
                        **inner = Document::load(&p.procedural_appearances[&new])
                            .map_err(|e| e.to_string())?;
                    } else {
                        document(inner, p, geo, mats, depth + 1)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn geometry(
        id: Uuid,
        p: &mut Project,
        geo: &mut BTreeMap<Uuid, Uuid>,
        mats: &mut BTreeMap<Uuid, Uuid>,
        depth: usize,
    ) -> Result<Uuid, String> {
        if let Some(new) = geo.get(&id) {
            return Ok(*new);
        }
        let mut doc = Document::load(
            p.procedural_appearances
                .get(&id)
                .ok_or("Model is missing")?,
        )
        .map_err(|e| e.to_string())?;
        let new = Uuid::new_v4();
        geo.insert(id, new);
        document(&mut doc, p, geo, mats, depth)?;
        doc.label = format!("{} variant", doc.label);
        p.procedural_appearances
            .insert(new, doc.save().map_err(|e| e.to_string())?);
        Ok(new)
    }
    for asset in assets(owner, p)
        .into_iter()
        .filter(|a| a.kind == Kind::Geometry)
    {
        let doc =
            Document::load(&p.procedural_appearances[&asset.id]).map_err(|e| e.to_string())?;
        shared::appearance_assets::resolve(p, &doc)?;
    }
    let mut candidate = p.clone();
    let mut geo = BTreeMap::new();
    let mut mats = BTreeMap::new();
    let mut data = owner
        .data(p)
        .ok_or("Appearance owner was removed")?
        .to_owned();
    if matches!(owner, Owner::Character(_)) {
        let old = body_source(owner, p);
        let id = geometry_id(&old)
            .ok_or("This character uses a bitmap avatar; choose a procedural model first")?;
        let new = geometry(id, &mut candidate, &mut geo, &mut mats, 0)?;
        let profile = attribute(&data, "appearance_profile").unwrap_or(old);
        data = with_attribute(&data, "appearance_profile", &profile)?;
        data = bind_character_model(&data, &new.to_string())?;
    } else {
        for key in [
            "appearance_geometry",
            "appearance_geometry_off",
            "appearance_worn_geometry",
        ] {
            if let Some(id) = attribute(&data, key).and_then(|s| geometry_id(&s)) {
                let new = geometry(id, &mut candidate, &mut geo, &mut mats, 0)?;
                data = with_attribute(&data, key, &new.to_string())?;
            }
        }
        for (on, key) in [(true, "appearance_icon"), (false, "appearance_icon_off")] {
            if let Some(id) = shared::ruleset_materials::icon_id_for_state(&data, on) {
                let new = material(id, &mut candidate, &mut mats)?;
                data = with_attribute(&data, key, &new.to_string())?;
            }
        }
        if let Some(table) = data
            .parse::<toml::Table>()
            .ok()
            .and_then(|t| t.get("attributes").cloned())
        {
            if let Some(entries) = table.get("appearance_materials").and_then(|v| v.as_array()) {
                let mut new = vec![];
                for entry in entries.iter().filter_map(|v| v.as_str()) {
                    if let Some((part, source)) = entry.split_once('=') {
                        let id = source
                            .strip_prefix("ruleset:material/")
                            .map(shared::rulesets::material_asset_id)
                            .or_else(|| Uuid::parse_str(source).ok())
                            .ok_or("Invalid material source")?;
                        new.push(format!(
                            "{part}={}",
                            material(id, &mut candidate, &mut mats)?
                        ));
                    }
                }
                let mut doc = data
                    .parse::<toml_edit::DocumentMut>()
                    .map_err(|e| e.to_string())?;
                let mut array = toml_edit::Array::new();
                for v in new {
                    array.push(v);
                }
                doc["attributes"]["appearance_materials"] = toml_edit::value(array);
                data = doc.to_string();
            }
        }
    }
    owner.set_data(&mut candidate, data)?;
    *p = candidate;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Project {
        static PROJECT: std::sync::LazyLock<Project> = std::sync::LazyLock::new(|| {
            let mut p = Project::default();
            p.sync_ruleset_items().unwrap();
            p
        });
        PROJECT.clone()
    }
    #[test]
    fn project_tree_groups_items_and_has_one_appearance_entry() {
        let p = fixture();
        let mut root = TheTreeNode::new(TheId::named("Items"));
        crate::undo::project_helper::populate_item_tree(&mut root, &p);
        assert!(root.childs.len() > 1 && root.childs.len() < p.items.len());
        let torch = p
            .items
            .values()
            .find(|i| attribute(&i.data, "ruleset_id").as_deref() == Some("torch"))
            .unwrap();
        assert_eq!(
            crate::undo::project_helper::item_tree_category(torch),
            "Utility"
        );
        assert!(
            root.childs
                .iter()
                .find(|g| g.id.name == "Utility")
                .unwrap()
                .childs
                .iter()
                .any(|n| n.id.uuid == torch.id)
        );
        let leaves = root
            .childs
            .iter()
            .flat_map(|g| &g.childs)
            .collect::<Vec<_>>();
        assert_eq!(leaves.len(), p.items.len());
        for leaf in leaves {
            let names = leaf
                .widgets
                .iter()
                .map(|w| w.id().name.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                names,
                vec![
                    "Item Item",
                    "Item Appearance",
                    "Item Item Code Edit",
                    "Item Item Data Edit"
                ]
            );
            assert!(p.items.contains_key(&leaf.id.uuid));
        }
        let mut custom = Item::new();
        custom.name = "Custom test item".into();
        custom.data.clear();
        crate::undo::project_helper::append_item_tree(&mut root, &custom, &p, true);
        let category = root
            .childs
            .iter()
            .find(|g| g.id.name == "Custom Items")
            .unwrap();
        assert!(category.open && category.childs.iter().any(|n| n.id.uuid == custom.id));
        crate::undo::project_helper::remove_item_tree(&mut root, &custom.id);
        assert!(root.childs.iter().all(|g| !g.childs.is_empty()));
        let character = Character::new();
        let node = crate::undo::project_helper::gen_character_tree_node(&character);
        assert_eq!(
            node.widgets
                .iter()
                .filter(|w| w.id().name == "Character Appearance")
                .count(),
            1
        );
    }
    #[test]
    fn icon_states_worn_geometry_and_materials_are_accessible_from_one_owner() {
        let p = fixture();
        let torch = p
            .items
            .values()
            .find(|i| attribute(&i.data, "ruleset_id").as_deref() == Some("torch"))
            .unwrap();
        let list = assets(Owner::Item(torch.id), &p);
        assert!(list.iter().any(|a| a.label == "Icon · active"));
        assert!(list.iter().any(|a| a.label == "Icon · inactive"));
        assert!(list.iter().filter(|a| a.kind == Kind::Geometry).count() >= 2);
        let shirt = p
            .items
            .values()
            .find(|i| attribute(&i.data, "ruleset_id").as_deref() == Some("linen_shirt"))
            .unwrap();
        let list = assets(Owner::Item(shirt.id), &p);
        assert!(list.iter().any(|a| a.label == "Worn model"));
        assert!(list.iter().any(|a| a.kind == Kind::Material));
    }
    #[test]
    fn orc_variant_retains_bindings_and_shared_assets_and_survives_save() {
        let mut p = fixture();
        let mut c = Character::new();
        c.name = "Custom Orc".into();
        c.data = "# Preserve behavior\n[attributes]\navatar = 'orc'\nSTR = 17\n".into();
        let sword = p
            .items
            .values()
            .find(|i| attribute(&i.data, "ruleset_id").as_deref() == Some("training_sword"))
            .unwrap()
            .id;
        c.preview_rigging = format!("animation = 'Walk'\n[slots]\nmain_hand = '{sword}'\n");
        let id = c.id;
        p.characters.insert(id, c);
        let old_geometry = p.procedural_appearances.clone();
        let old_materials = p.procedural_materials.clone();
        make_variant(Owner::Character(id), &mut p).unwrap();
        let source = body_source(Owner::Character(id), &p);
        assert!(Uuid::parse_str(&source).is_ok());
        assert_eq!(
            attribute(&p.characters[&id].data, "appearance_profile").as_deref(),
            Some("orc")
        );
        assert!(p.characters[&id].data.contains("# Preserve behavior"));
        assert!(p.characters[&id].data.contains("STR = 17"));
        for (id, text) in old_geometry {
            assert_eq!(&p.procedural_appearances[&id], &text);
        }
        for (id, text) in old_materials {
            assert_eq!(&p.procedural_materials[&id], &text);
        }
        let doc =
            Document::load(&p.procedural_appearances[&Uuid::parse_str(&source).unwrap()]).unwrap();
        assert!(
            shared::appearance_assets::resolve(&p, &doc)
                .unwrap()
                .evaluate()
                .unwrap()
                .mesh
                .rig
                .is_some()
        );
        let saved = serde_json::to_string(&p).unwrap();
        let reopened: Project = serde_json::from_str(&saved).unwrap();
        assert_eq!(body_source(Owner::Character(id), &reopened), source);
        assert!(
            reopened.characters[&id]
                .preview_rigging
                .contains(&sword.to_string())
        );
        assert!(
            assets(Owner::Character(id), &reopened)
                .iter()
                .any(|a| a.kind == Kind::Material)
        );
    }
    #[test]
    fn item_variant_copies_icon_and_materials_without_modifying_other_items() {
        let mut p = fixture();
        let item = p
            .items
            .values()
            .find(|i| attribute(&i.data, "ruleset_id").as_deref() == Some("linen_shirt"))
            .unwrap()
            .id;
        let old = p.clone();
        let before = assets(Owner::Item(item), &p);
        make_variant(Owner::Item(item), &mut p).unwrap();
        let after = assets(Owner::Item(item), &p);
        assert!(!after.is_empty());
        for asset in after {
            assert!(!before.iter().any(|a| a.id == asset.id));
        }
        for (id, item_before) in &old.items {
            if *id != item {
                assert_eq!(p.items[id].data, item_before.data);
            }
        }
        for (id, source) in &old.procedural_materials {
            assert_eq!(&p.procedural_materials[id], source);
        }
    }
    #[test]
    fn switching_from_a_bitmap_avatar_removes_conflicting_source_attributes() {
        let data = bind_character_model(
            "[attributes]\navatar_id='old-id'\ntile_id='old-tile'\nsource='old'\nHP=20\n",
            "orc",
        )
        .unwrap();
        assert_eq!(attribute(&data, "avatar").as_deref(), Some("orc"));
        assert!(!data.contains("avatar_id"));
        assert!(!data.contains("tile_id"));
        assert!(data.contains("HP=20"));
    }
}
