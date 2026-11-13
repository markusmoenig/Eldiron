//! Source-scoped overlay interaction, separate from world gestures/rendering.
use super::*;
use crate::hud::{HudAction, HudEffects, HudHit};
use std::sync::Arc;

pub enum ToolOverlayRoute {
    Unavailable,
    Outside {
        redraw: bool,
    },
    Consumed {
        redraw: bool,
        command: Option<ToolOverlayCommand>,
    },
}
#[derive(Default)]
pub struct ToolOverlayInputState {
    capture: Option<ToolOverlayTarget>,
}
enum Action {
    Hud(HudAction),
    Terrain(terrain_overlay::Action),
    Props {
        rect: TheDim,
        action: props_overlay::Action,
    },
}
/// The accepted painted frame stays alive until delivery. Model changes are
/// revalidated when applying the command, rather than applying a stale callback
/// to whichever document/selection happens to be active later.
pub struct ToolOverlayCommand {
    target: ToolOverlayTarget,
    frame: Arc<ToolOverlayFrame>,
    action: Action,
}
pub struct ScopedToolOverlayEvent {
    target: ToolOverlayTarget,
    frame: Arc<ToolOverlayFrame>,
    event: Option<TheEvent>,
    authoring: bool,
}
pub struct ToolOverlayEffects {
    pub scene_dirty: bool,
    pub overlay_dirty: bool,
    pub redraw: bool,
    pub events: Vec<ScopedToolOverlayEvent>,
}
impl ToolOverlayEffects {
    fn empty() -> Self {
        Self {
            scene_dirty: false,
            overlay_dirty: false,
            redraw: false,
            events: vec![],
        }
    }
}
impl ScopedToolOverlayEvent {
    /// Consuming delivery is exactly-once, including rejected events. Generic
    /// notifications need the accepted owner lifetime; authoring events also
    /// require their immutable source to remain current.
    pub(crate) fn take_if_current<T: Tool + ?Sized>(
        mut self,
        tool: &mut T,
        target: ToolOverlayTarget,
        map: &Map,
        server: &ServerContext,
    ) -> Option<TheEvent> {
        if self.target != target
            || !self.frame.matches(target, server)
            || tool.id().uuid != target.tool
        {
            return None;
        }
        if self.authoring {
            if !self.frame.command_source_current(tool, target, map, server) {
                return None;
            }
        } else if !tool
            .overlay_hud()
            .is_some_and(|hud| self.frame.hud_owner_matches(hud))
        {
            return None;
        }
        self.event.take()
    }
}
impl ToolOverlayCommand {
    pub(crate) fn apply<T: Tool + ?Sized>(
        self,
        tool: &mut T,
        target: ToolOverlayTarget,
        map: &mut Map,
        server: &mut ServerContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Option<ToolOverlayEffects> {
        if self.target != target || !self.frame.command_source_current(tool, target, map, server) {
            return None;
        }
        let mut effects = ToolOverlayEffects::empty();
        match &self.action {
            Action::Terrain(action) => {
                if !tool.apply_terrain_overlay(*action, map, server, ctx) {
                    return None;
                }
                effects.redraw = true;
                effects.overlay_dirty = true;
            }
            Action::Hud(action) => {
                let HudEffects {
                    scene_dirty,
                    overlay_dirty,
                    redraw,
                    events,
                } = tool.overlay_hud()?.apply_action(*action, map, server);
                effects.scene_dirty = scene_dirty;
                effects.overlay_dirty = overlay_dirty;
                effects.redraw = redraw;
                effects.events = events
                    .into_iter()
                    .map(|event| ScopedToolOverlayEvent {
                        target,
                        frame: self.frame.clone(),
                        event: Some(event),
                        authoring: false,
                    })
                    .collect();
            }
            Action::Props {
                action: props_overlay::Action::Event(event),
                ..
            } => {
                effects.redraw = true;
                effects.events.push(ScopedToolOverlayEvent {
                    target,
                    frame: self.frame.clone(),
                    event: Some(event.clone()),
                    authoring: true,
                });
            }
            Action::Props { .. } => {
                if !tool.show_overlay_popup(&self, server, ui, ctx) {
                    return None;
                }
                effects.redraw = true;
            }
        }
        Some(effects)
    }
    pub(crate) fn show_props_popup(
        &self,
        overlay: &props_overlay::PropsOverlay,
        server: &ServerContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        match &self.action {
            Action::Props { rect, action }
                if matches!(
                    action,
                    props_overlay::Action::Placement | props_overlay::Action::Controls
                ) =>
            {
                overlay.dispatch_action(*rect, action.clone(), ui, ctx, server);
                true
            }
            _ => false,
        }
    }
}
impl ToolOverlayInputState {
    pub(crate) fn invalidate(&mut self) {
        self.capture = None;
    }
    pub(crate) fn route<T: Tool + ?Sized>(
        &mut self,
        tool: &mut T,
        frame: Option<&Arc<ToolOverlayFrame>>,
        target: ToolOverlayTarget,
        event: &MapEvent,
        focused: bool,
        map: &Map,
        server: &ServerContext,
    ) -> ToolOverlayRoute {
        if matches!(event, MapEvent::MapEscape) {
            self.capture = None;
            return ToolOverlayRoute::Outside { redraw: false };
        }
        if matches!(event, MapEvent::MapDragged(_) | MapEvent::MapUp(_)) {
            let Some(capture) = self.capture else {
                return ToolOverlayRoute::Outside { redraw: false };
            };
            if matches!(event, MapEvent::MapUp(_)) {
                self.capture = None;
            }
            if capture != target || !target.valid(server) || tool.id().uuid != target.tool {
                self.capture = None;
                return ToolOverlayRoute::Unavailable;
            }
            return ToolOverlayRoute::Consumed {
                redraw: false,
                command: None,
            };
        }
        let key = match event {
            MapEvent::MapKey(key) if focused => Hud::shortcut_action(*key),
            _ => None,
        };
        let point = match event {
            MapEvent::MapClicked(at) | MapEvent::MapHover(at) => Some(*at),
            _ => None,
        };
        let terrain_key = match event {
            MapEvent::MapKey(key) if focused && frame.is_some_and(|frame| frame.has_terrain()) => {
                terrain_overlay::shortcut(*key)
            }
            _ => None,
        };
        if point.is_none() && key.is_none() && terrain_key.is_none() {
            return ToolOverlayRoute::Outside { redraw: false };
        }
        if matches!(event, MapEvent::MapClicked(_)) {
            self.capture = None;
        }
        let Some(frame) = frame else {
            return ToolOverlayRoute::Unavailable;
        };
        if !frame.input_current(tool, target, map, server) {
            return ToolOverlayRoute::Unavailable;
        }
        if let Some(action) = terrain_key {
            return ToolOverlayRoute::Consumed {
                redraw: true,
                command: Some(ToolOverlayCommand {
                    target,
                    frame: frame.clone(),
                    action: Action::Terrain(action),
                }),
            };
        }
        if let Some(action) = key
            && frame.has_hud()
        {
            return ToolOverlayRoute::Consumed {
                redraw: true,
                command: Some(ToolOverlayCommand {
                    target,
                    frame: frame.clone(),
                    action: Action::Hud(action),
                }),
            };
        }
        let Some(at) = point else {
            return ToolOverlayRoute::Outside { redraw: false };
        };
        if at.x < 0 || at.y < 0 || at.x as usize >= target.size.0 || at.y as usize >= target.size.1
        {
            return ToolOverlayRoute::Outside { redraw: false };
        }
        let redraw = matches!(event, MapEvent::MapHover(_))
            && tool
                .overlay_hud()
                .is_some_and(|hud| hud.update_pointer_position(at));
        let clicked = matches!(event, MapEvent::MapClicked(_));
        // Reverse painter order: an interactive foreground part wins over the HUD.
        for part in frame.parts.iter().rev() {
            if let Part::Terrain(panel) = part
                && panel.bounds.contains(at)
            {
                if clicked {
                    self.capture = Some(target);
                }
                let action = if clicked {
                    panel
                        .buttons
                        .iter()
                        .find(|(rect, _)| rect.contains(at))
                        .map(|(_, action)| *action)
                        .unwrap_or(terrain_overlay::Action::ClearHover)
                } else {
                    terrain_overlay::Action::ClearHover
                };
                return ToolOverlayRoute::Consumed {
                    redraw: true,
                    command: Some(ToolOverlayCommand {
                        target,
                        frame: frame.clone(),
                        action: Action::Terrain(action),
                    }),
                };
            }
            if let Part::Props(props) = part
                && props.bounds.contains(at)
            {
                if clicked {
                    self.capture = Some(target);
                }
                let action = props
                    .buttons
                    .iter()
                    .find(|(rect, _)| rect.contains(at))
                    .cloned();
                return ToolOverlayRoute::Consumed {
                    redraw: redraw || clicked,
                    command: if clicked {
                        action.map(|(rect, action)| ToolOverlayCommand {
                            target,
                            frame: frame.clone(),
                            action: Action::Props { rect, action },
                        })
                    } else {
                        None
                    },
                };
            }
        }
        let Some(hud) = tool.overlay_hud() else {
            return ToolOverlayRoute::Outside { redraw };
        };
        match hud.hit_test(target.hud_scope(), target.size, at, map, server) {
            HudHit::Unavailable => ToolOverlayRoute::Unavailable,
            HudHit::Outside => ToolOverlayRoute::Outside { redraw },
            HudHit::Action(action) => {
                if clicked {
                    self.capture = Some(target);
                }
                ToolOverlayRoute::Consumed {
                    redraw: redraw || clicked,
                    command: clicked.then(|| ToolOverlayCommand {
                        target,
                        frame: frame.clone(),
                        action: Action::Hud(action),
                    }),
                }
            }
        }
    }
}
