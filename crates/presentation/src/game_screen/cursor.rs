//! Source artwork is prepared independently of composition. Cursor/drag layers
//! use logical geometry and immutable sources, never a CPU overlay framebuffer.
use super::*;
use rusterix::client::widget::{ButtonVisualState, Widget};

#[derive(Default)]
pub(super) struct PointerVisuals {
    publication: Option<(Uuid, ContentRevision)>,
    default: Option<Uuid>,
    tile: Option<Uuid>,
    position: Option<[f32; 2]>,
    mark: Option<(String, Rect)>,
    drag: Option<Arc<PreparedContent>>,
}
fn cursor_exists(assets: &rusterix::Assets, id: Uuid) -> bool {
    assets
        .tiles
        .get(&id)
        .and_then(|tile| tile.textures.first())
        .is_some_and(|texture| texture.width > 0 && texture.height > 0)
}
impl GameScreenController {
    /// The host restores its own cursor whenever no valid custom source is
    /// prepared inside this screen, including focus/leave/zero-size changes.
    pub fn uses_custom_cursor(&self) -> bool {
        let position = self.pointer_visual_position();
        position.is_some()
            && position == self.pointer_visuals.position
            && self.pointer_visuals.tile.is_some()
    }
    pub(super) fn prepare_pointer_visuals(
        &mut self,
        world: &RuntimeView<'_>,
        revision: ContentRevision,
    ) -> Result<(), String> {
        let publication = (world.session_id, revision);
        if self.pointer_visuals.publication != Some(publication) {
            let config = world
                .config()
                .parse::<toml::Table>()
                .map_err(|error| error.to_string())?;
            self.pointer_visuals.default = config
                .get("viewport")
                .and_then(|viewport| viewport.get("cursor_id"))
                .and_then(toml::Value::as_str)
                .and_then(|id| Uuid::parse_str(id).ok())
                .filter(|id| cursor_exists(world.assets, *id));
            self.pointer_visuals.publication = Some(publication);
        }
        let position = self.pointer_visual_position();
        let default = self
            .pointer_visuals
            .default
            .filter(|id| cursor_exists(world.assets, *id));
        let mut tile = default;
        if let Some((entity, item, pressed)) = self.pointer_cursor_target(world) {
            if let Some(pair) = screen_model::active_intent_cursors(
                &self.elements.activated_widgets,
                &self.elements.button_widgets,
                &self.intent,
            )
            .and_then(|cursors| cursors.target(entity, item))
            {
                // A press without a clicked cursor uses the normal default,
                // matching the reference client instead of retaining hover.
                tile = (if pressed { pair.pressed } else { pair.normal })
                    .filter(|id| cursor_exists(world.assets, *id))
                    .or(default);
            }
        }
        let mark = if tile == default && self.pointer_targets_world(world) {
            position.and_then(|point| {
                let intent = screen_model::selected_intent(
                    &self.elements.activated_widgets,
                    &self.elements.button_widgets,
                    &self.intent,
                )?;
                let command =
                    screen_model::intent_cursor_command(&intent, self.hovered_item, world.map?)?;
                let size = self.elements.layout.viewport;
                Some((
                    command,
                    Rect::new(
                        (point[0] + 9.).clamp(0., (size.x as f32 - 16.).max(0.)),
                        (point[1] + 9.).clamp(0., (size.y as f32 - 16.).max(0.)),
                        16.,
                        16.,
                    ),
                ))
            })
        } else {
            None
        };
        let item = self.dragged_ui_item(world).or_else(|| {
            self.dragged_world_item(world)
                .and_then(|id| world.map?.items.iter().find(|item| item.id == id))
        });
        let drag = if let Some(item) = item {
            self.sources.prepare(
                ContentRecipe::Item {
                    item,
                    frame: self.animation_frame,
                },
                world.assets,
                revision,
            )?
        } else {
            None
        };
        self.pointer_visuals.position = position;
        self.pointer_visuals.tile = tile;
        self.pointer_visuals.mark = mark;
        self.pointer_visuals.drag = drag;
        Ok(())
    }
}
pub(super) fn paint(
    p: &mut ScreenPainter<'_>,
    visuals: &PointerVisuals,
    assets: &rusterix::Assets,
    frame: usize,
) -> Result<(), String> {
    let Some(point) = visuals.position else {
        return Ok(());
    };
    if let Some(source) = &visuals.drag {
        let image = source.resolve(p.images)?;
        p.list.ops.push(PaintOp::Image {
            image,
            rect: Uv::new((point[0] - 14.).max(0.), (point[1] - 14.).max(0.), 28., 28.),
            uv: Uv::new(0., 0., 1., 1.),
            sampling: Sampling::Nearest,
            tint: [1.; 4],
        });
    }
    if let Some((id, texture)) = visuals.tile.and_then(|id| {
        assets
            .tiles
            .get(&id)
            .and_then(|tile| tile.textures.first())
            .map(|texture| (id, texture))
    }) {
        p.texture(
            &format!("cursor/{id}"),
            texture,
            Rect::new(
                point[0] - texture.width as f32 / 2.,
                point[1] - texture.height as f32 / 2.,
                texture.width as f32,
                texture.height as f32,
            ),
            Uv::new(0., 0., 1., 1.),
            Sampling::Nearest,
            [1.; 4],
        )?;
    }
    if let Some((command, bounds)) = &visuals.mark {
        if let Some(texture) =
            Widget::command_icon_texture(assets, Some(command), ButtonVisualState::Normal, frame)
        {
            if texture.width > 0 && texture.height > 0 {
                let image = p.images.rgba(
                    &format!("texture/{:p}", texture),
                    texture.width as u32,
                    texture.height as u32,
                    &texture.data,
                )?;
                p.list.ops.push(PaintOp::ThresholdImage {
                    image,
                    rect: rect(*bounds),
                    uv: Uv::new(0., 0., 1., 1.),
                    sampling: Sampling::Nearest,
                    threshold: 100,
                    dark: color([35, 35, 35, 255]),
                    light: color([230, 230, 230, 255]),
                });
            }
        }
    }
    Ok(())
}
