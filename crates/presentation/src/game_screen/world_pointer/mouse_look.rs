//! Relative look owns a displayed view and a runtime player, never a surface or
//! SceneVM. Pitch belongs to its widget; yaw uses the shared CPU camera policy.
use super::*;
use rusterix::camera::mouse_look::MouseLook;

#[derive(Clone, Copy, PartialEq)]
pub(super) struct LookSettings {
    sensitivity: f32,
    invert_y: bool,
}
impl LookSettings {
    pub(super) fn from_widget(widget: &rusterix::client::widget::game::GameWidget) -> Option<Self> {
        (widget.camera == PlayerCamera::D3FirstPMouse).then_some(Self {
            sensitivity: if widget.mouse_sensitivity.is_finite() {
                widget.mouse_sensitivity.clamp(0.01, 5.)
            } else {
                0.2
            },
            invert_y: widget.mouse_invert_y,
        })
    }
}
pub(super) struct LookCapture {
    binding: Binding,
    view: Uuid,
    rect: Rect,
    player: u32,
    settings: LookSettings,
    look: MouseLook,
}
impl GameScreenController {
    /// Relative-motion eligibility is independent of whether the center ray
    /// hits geometry. Empty sky still permits camera control.
    pub fn mouse_look_view(&self) -> Option<Uuid> {
        self.world.look.as_ref().map(|capture| capture.view)
    }
    pub(super) fn refresh_mouse_look(&mut self, world: &RuntimeView<'_>, blocked: bool) {
        let eligible = (|| {
            if blocked
                || self
                    .world
                    .capture
                    .as_ref()
                    .is_some_and(|capture| capture.cancelled || capture.item.is_some())
            {
                return None;
            }
            let view = self.pointer_view(world, self.world.pointer?).ok()?;
            let settings = view.look?;
            let widget = self.elements.game_widgets.get(&view.key)?;
            // Edited settings need their own acknowledged publication.
            if LookSettings::from_widget(widget) != Some(settings) {
                return None;
            }
            let player = world.player_entity()?;
            Some((
                self.world.displayed.as_ref()?.binding,
                view.key,
                view.rect,
                player.id,
                player.orientation,
                settings,
                widget.mouse_look_pitch,
            ))
        })();
        let Some((binding, view, bounds, player, facing, settings, pitch)) = eligible else {
            self.world.look = None;
            return;
        };
        if self.world.look.as_ref().is_some_and(|capture| {
            capture.binding == binding
                && capture.view == view
                && capture.rect == bounds
                && capture.player == player
                && capture.settings == settings
        }) {
            return;
        }
        let mut look = MouseLook::default();
        look.pitch = if pitch.is_finite() {
            pitch.clamp(-85., 85.)
        } else {
            0.
        };
        look.begin(facing, settings.sensitivity, settings.invert_y);
        self.world.look = Some(LookCapture {
            binding,
            view,
            rect: bounds,
            player,
            settings,
            look,
        });
    }
    pub(super) fn mouse_look_motion(
        &mut self,
        world: &RuntimeView<'_>,
        delta: [f32; 2],
        blocked: bool,
    ) -> ScreenInputResponse {
        self.refresh_mouse_look(world, blocked);
        let Some(capture) = &mut self.world.look else {
            return ScreenInputResponse::default();
        };
        if !delta.iter().all(|value| value.is_finite()) {
            return ScreenInputResponse::default();
        }
        if delta == [0., 0.] {
            return ScreenInputResponse::handled(false);
        }
        if capture.look.motion(Vec2::new(delta[0], delta[1])).is_none() {
            return ScreenInputResponse::default();
        }
        self.elements
            .game_widgets
            .get_mut(&capture.view)
            .unwrap()
            .mouse_look_pitch = capture.look.pitch;
        ScreenInputResponse {
            effects: if delta[0] != 0. {
                vec![ScreenEffect::Action(EntityAction::LookYaw(
                    delta[0] * capture.look.sensitivity,
                ))]
            } else {
                Vec::new()
            },
            ..ScreenInputResponse::handled(true)
        }
    }
}
