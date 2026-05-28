use crate::ScepterCommand;
use serde::{Deserialize, Serialize};

/// A named group of Scepter commands that can be validated, previewed, and applied together.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScepterPlan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub commands: Vec<ScepterCommand>,
}

impl ScepterPlan {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Self::default()
        }
    }

    pub fn push(&mut self, command: ScepterCommand) {
        self.commands.push(command);
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        RegionPaintRect, RegionRef, ScepterCommand, ScepterLorebook, ScepterPlan, TileSelector,
    };

    #[test]
    fn command_round_trips_with_stable_protocol_name() {
        let command = ScepterCommand::RegionPaintRect(RegionPaintRect {
            region: RegionRef::name("Harbor"),
            tile: TileSelector::alias("stone_floor_dark"),
            rect: [4, 4, 12, 8],
            layer: Some("ai.generated".to_string()),
            select: None,
            replace_existing: None,
        });

        let json = serde_json::to_string(&command).unwrap();
        assert!(json.contains("\"command\":\"region.paint_rect\""));

        let decoded: ScepterCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.name(), "region.paint_rect");
        assert_eq!(decoded, command);
    }

    #[test]
    fn lorebook_contains_first_slice_commands() {
        let lorebook = ScepterLorebook::built_in();
        for command in [
            "scepter.describe_command",
            "region.snapshot",
            "region.summary",
            "region.paint_rect",
            "region.create_sector",
            "region.place_item",
            "region.place_character",
            "tile.contact_sheet",
            "tile.set_meta",
            "tile_group.create",
            "tileset.import_batch",
            "script.validate",
            "geometry.create_room",
        ] {
            assert!(
                lorebook.describe_command(command).is_some(),
                "missing {command}"
            );
        }

        let paint = lorebook.describe_command("region.paint_rect").unwrap();
        assert!(paint.previewable);
        assert!(paint.undoable);
        assert!(!paint.examples.is_empty());
    }

    #[test]
    fn plan_serializes_as_command_list() {
        let mut plan = ScepterPlan::new("Small Stone Room");
        plan.push(ScepterCommand::RegionPaintRect(RegionPaintRect {
            region: RegionRef::name("Harbor"),
            tile: TileSelector::style_kind("stone", "floor"),
            rect: [0, 0, 8, 6],
            layer: None,
            select: None,
            replace_existing: None,
        }));

        let json = serde_json::to_value(&plan).unwrap();
        assert_eq!(json["name"], "Small Stone Room");
        assert_eq!(json["commands"][0]["command"], "region.paint_rect");
    }
}
