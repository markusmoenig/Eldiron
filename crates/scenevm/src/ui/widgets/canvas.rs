use crate::ui::{
    event::{UiEvent, UiEventOutcome},
    workspace::{UiView, ViewContext},
};

/// A container widget that can hold other widgets and be shown/hidden as a group.
/// Perfect for organizing widgets by app mode/screen.
#[derive(Debug, Clone)]
pub struct Canvas {
    pub id: String,
    pub visible: bool,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            id: String::new(),
            visible: true,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl UiView for Canvas {
    fn build(&mut self, _ctx: &mut ViewContext) {
        // Canvas itself doesn't render anything - it's just a container.
        // Its children are rendered by the workspace if the canvas is visible.
    }

    fn handle_event(&mut self, _evt: &UiEvent) -> UiEventOutcome {
        // Canvas doesn't handle events directly - events pass through to children
        UiEventOutcome::none()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn view_id(&self) -> &str {
        &self.id
    }
}
