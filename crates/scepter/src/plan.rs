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
        ActionRun, ActionRunScript, RegionPaintRect, RegionRef, ScepterCommand, ScepterLorebook,
        ScepterPlan, TileSelector, ToolSelect,
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
            "tileset.import_batch",
            "script.validate",
            "action.list",
            "action.run",
            "action.run_script",
            "tool.list",
            "tool.select",
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
    fn action_commands_round_trip_with_stable_protocol_names() {
        let direct = ScepterCommand::ActionRun(ActionRun {
            id: "face.extrude".to_string(),
            parameters_toml: "amount = 2".to_string(),
        });
        let direct_json = serde_json::to_string(&direct).unwrap();
        let decoded: ScepterCommand = serde_json::from_str(&direct_json).unwrap();
        assert_eq!(decoded, direct);
        assert_eq!(decoded.name(), "action.run");

        let scripted = ScepterCommand::ActionRunScript(ActionRunScript {
            source: r#"editor_action("camera.isometric", "");"#.to_string(),
        });
        let scripted_json = serde_json::to_string(&scripted).unwrap();
        let decoded: ScepterCommand = serde_json::from_str(&scripted_json).unwrap();
        assert_eq!(decoded, scripted);
        assert_eq!(decoded.name(), "action.run_script");

        let tool = ScepterCommand::ToolSelect(ToolSelect {
            id: "tool.geometry".to_string(),
        });
        let tool_json = serde_json::to_string(&tool).unwrap();
        let decoded: ScepterCommand = serde_json::from_str(&tool_json).unwrap();
        assert_eq!(decoded, tool);
        assert_eq!(decoded.name(), "tool.select");
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

#[cfg(test)]
mod request_tests {
    use crate::*;
    #[test]
    fn edit_context_and_transaction_roundtrip() {
        let request: ScepterRequest=serde_json::from_value(serde_json::json!({
            "request_id":"road-1","session":"session-uuid","revision":"123",
            "command":"project.transaction","params":{"operations":[{"op":"composition_copy","params":{"id":"new-uuid","source":"old-uuid"}}]}
        })).unwrap();
        assert!(request.command.is_mutation());
        assert_eq!(
            serde_json::from_str::<ScepterRequest>(&serde_json::to_string(&request).unwrap())
                .unwrap(),
            request
        );
        let catalog = ScepterLorebook::built_in();
        assert!(
            catalog
                .describe_command("project.transaction")
                .unwrap()
                .implemented
        );
        assert!(
            catalog
                .describe_command("region.render_preview")
                .unwrap()
                .implemented
        );
        assert!(
            catalog
                .describe_command("geometry.create_room")
                .unwrap()
                .implemented
        );
    }
}

#[cfg(test)]
mod world_protocol_tests {
    use crate::*;
    use serde_json::json;
    #[test]
    fn map_collaboration_requests_are_discoverable_and_roundtrip() {
        let catalog = ScepterLorebook::built_in();
        for name in [
            "editor.context",
            "editor.select",
            "editor.focus",
            "project.changes",
            "entity.templates",
            "region.entities",
            "geometry.create_room",
            "region.place_item",
        ] {
            assert!(
                catalog.describe_command(name).unwrap().implemented,
                "{name}"
            );
        }
        for command in [
            json!({"command":"editor.select","params":{"targets":{"walls":["wall"]},"wall_spans":["span"],"faces":[],"geometry_vertices":[]}}),
            json!({"command":"editor.focus","params":{"view":"2d","center":[1,0,2],"scale":0.45}}),
            json!({"command":"project.changes","params":{"after":7,"limit":64}}),
            json!({"command":"project.transaction","params":{"operations":[
                {"op":"room_create","params":{"region":{"name":"Harbor"},"id":"wall","name":"Workshop","min":[0,0,0],"max":[6,3,5],"thickness":0.25,"doors":[{"wall":0,"center":3,"width":1.2,"height":2.2}]}},
                {"op":"map_copy","params":{"region":{"name":"Harbor"},"targets":{"walls":["wall"]},"translation":[10,0,0]}},
                {"op":"entity_place","params":{"region":{"name":"Harbor"},"id":"torch","kind":"item","template":"Torch","position":[1,0,1],"active":true}}
            ]}}),
            json!({"command":"region.render_preview","params":{"region":{"name":"Harbor"},"editor_view":true,"zoom":0.45,"camera":{"center":[0,0,0],"azimuth":45,"elevation":55,"fov":60,"pitch":0}}}),
        ] {
            let parsed: ScepterCommand = serde_json::from_value(command).unwrap();
            let encoded = serde_json::to_value(&parsed).unwrap();
            assert_eq!(
                parsed,
                serde_json::from_value::<ScepterCommand>(encoded).unwrap()
            );
        }
        let read: ScepterCommand =
            serde_json::from_value(json!({"command":"project.changes","params":{}})).unwrap();
        assert!(!read.is_mutation());
    }
}
