use crate::{editor::DOCKMANAGER, prelude::*};

pub struct Minimize {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for Minimize {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_minimize_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_minimize")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_minimize_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::CTRLCMD, ']'))
    }

    fn is_applicable(
        &self,
        _map: &Map,
        _ctx: &mut TheContext,
        _server_ctx: &ServerContext,
    ) -> bool {
        DOCKMANAGER.read().unwrap().get_state() != DockManagerState::Minimized
    }

    fn apply(
        &self,
        _map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        DOCKMANAGER.write().unwrap().minimize(ui, ctx);

        None
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
