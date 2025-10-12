use crate::{
    editor::{CODEEDITOR, CODEGRIDFX},
    prelude::*,
};

pub struct ApplyTile {
    id: TheId,
}

impl Action for ApplyTile {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Apply Tile"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        str!("Apply Tile (Ctrl + A). Applies the current tile to the selected geometry.")
    }

    fn accel(&self) -> Option<char> {
        Some('A')
    }

    fn is_applicable(&self, map: &Map, ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        true
    }

    fn params(&self) -> TheNodeUI {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::Text(
            "softRigName".into(),
            "Rig Name".into(),
            "Set the name of the soft rig keyframe.".into(),
            "wdew".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        nodeui
    }
}
