//! CPU preparation of local-player registration shared by graphical and text
//! hosts. Authoring maps/templates remain immutable; submission is the session's job.
use crate::{Assets, Command, Map, Value};

#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerStartOptions<'a> {
    pub class: Option<&'a str>,
    pub name: Option<&'a str>,
    /// Explicit game-start commands may spawn at the named entrance; automatic
    /// registration preserves the authored position.
    pub entrance: bool,
}
#[derive(Clone, Debug)]
pub struct PlayerStartInfo {
    pub template: String,
    pub position: [f32; 2],
    pub input_bindings: Option<String>,
}
pub struct PreparedPlayerStart {
    pub command: Command,
    pub info: PlayerStartInfo,
}

pub fn prepare_player_start(
    assets: &Assets,
    map: &Map,
    options: PlayerStartOptions<'_>,
) -> Result<PreparedPlayerStart, String> {
    let templates = assets
        .entities
        .iter()
        .filter_map(|(name, (_, data))| {
            let data = data.parse::<toml::Table>().ok()?;
            data.get("attributes")?
                .get("player")?
                .as_bool()?
                .then_some(name.clone())
        })
        .collect::<Vec<_>>();
    prepare_player_start_with_templates(map, &templates, options)
}

/// Legacy controllers may already own a validated template catalog. Keep that
/// metadata input explicit while sharing the same entity/selection preparation.
pub(crate) fn prepare_player_start_with_templates(
    map: &Map,
    templates: &[String],
    options: PlayerStartOptions<'_>,
) -> Result<PreparedPlayerStart, String> {
    let (mut entity, template) = map
        .entities
        .iter()
        .find_map(|entity| {
            let name = entity.get_attr_string("class_name")?;
            templates.contains(&name).then(|| (entity.clone(), name))
        })
        .ok_or_else(|| format!("No player entity template found in region '{}'", map.name))?;
    if let Some(class) = options.class.map(str::trim).filter(|v| !v.is_empty()) {
        entity.set_attribute("_start_class", Value::Str(class.into()));
    }
    if let Some(name) = options.name.map(str::trim).filter(|v| !v.is_empty()) {
        entity.set_attribute("_start_name", Value::Str(name.into()));
    }
    if options.entrance
        && let Some(position) = map.named_area_center("entrance")
    {
        entity.set_pos_xz(position);
    }
    entity.set_attribute("player", Value::Bool(true));
    let position = entity.get_pos_xz();
    let info = PlayerStartInfo {
        template,
        position: [position.x, position.y],
        input_bindings: entity
            .attributes
            .get_str("_input_bindings")
            .map(str::to_owned),
    };
    Ok(PreparedPlayerStart {
        command: Command::CreateEntity(map.id, entity),
        info,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn player_selection_prepares_a_private_command_and_preserves_authored_defaults() {
        let mut assets = Assets::default();
        assets.entities.insert(
            "Hero".into(),
            (Default::default(), "[attributes]\nplayer=true".into()),
        );
        let mut map = Map::default();
        map.name = "Start".into();
        let mut actor = crate::Entity::default();
        actor
            .attributes
            .set("class_name", Value::Str("Hero".into()));
        actor
            .attributes
            .set("_input_bindings", Value::Str("Custom bindings".into()));
        actor.position.x = 12.;
        actor.position.z = 6.;
        map.entities.push(actor);
        let prepared = prepare_player_start(
            &assets,
            &map,
            PlayerStartOptions {
                class: Some("  Mage "),
                name: Some(" Éowyn "),
                entrance: false,
            },
        )
        .unwrap();
        assert_eq!(prepared.info.template, "Hero");
        assert_eq!(prepared.info.position, [12., 6.]);
        assert_eq!(
            prepared.info.input_bindings.as_deref(),
            Some("Custom bindings")
        );
        let Command::CreateEntity(id, actor) = prepared.command;
        assert_eq!(id, map.id);
        assert_eq!(actor.attributes.get_str("_start_class"), Some("Mage"));
        assert_eq!(actor.attributes.get_str("_start_name"), Some("Éowyn"));
        assert!(actor.is_player());
        assert!(map.entities[0].attributes.get("_start_class").is_none());
        assert!(map.entities[0].attributes.get("_start_name").is_none());
        let defaults = prepare_player_start(&assets, &map, PlayerStartOptions::default()).unwrap();
        let Command::CreateEntity(_, actor) = defaults.command;
        assert!(actor.attributes.get("_start_class").is_none());
        assert!(actor.attributes.get("_start_name").is_none());
    }
}
