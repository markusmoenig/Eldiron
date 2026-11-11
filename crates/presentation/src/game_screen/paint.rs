use super::*;
use theframework::paint::{FillRule, Path, PathElement};

fn geometry(
    p: &mut ScreenPainter<'_>,
    scene: &Scene,
    layout: ScreenLayout,
    assets: &rusterix::Assets,
) -> Result<(), String> {
    let (x, y) = rusterix::utils::align_screen_to_grid(
        layout.viewport.x as f32,
        layout.viewport.y as f32,
        layout.grid_size,
    );
    p.list.ops.push(PaintOp::PushTransform(
        Transform::scale(layout.grid_size, layout.grid_size).then(Transform::translate(
            -x * layout.grid_size,
            -y * layout.grid_size,
        )),
    ));
    let result = p.unlit_scene(scene, assets);
    p.list.ops.push(PaintOp::PopTransform);
    result
}
fn rect_path(path: &mut Path, r: Rect) {
    path.elements.extend([
        PathElement::Move([r.x, r.y]),
        PathElement::Line([r.x + r.width, r.y]),
        PathElement::Line([r.x + r.width, r.y + r.height]),
        PathElement::Line([r.x, r.y + r.height]),
        PathElement::Close,
    ]);
}
impl GameScreenController {
    /// Emit the prepared UI and opaque scene handles. This neither updates the
    /// authoritative session nor queues messages or generates avatar/item pixels.
    pub fn paint(
        &mut self,
        world: &RuntimeView<'_>,
        fonts: &mut GameFonts,
        images: &mut dyn ScreenImages,
    ) -> Result<PaintList, String> {
        if self.font_version != Some(fonts.version()) {
            return Err("Prepare screen layouts after changing their font owner".into());
        }
        let geometry_cache = self
            .geometry
            .as_mut()
            .ok_or("Prepare screen layout before painting")?;
        geometry_cache.base.animation_frame = self.animation_frame;
        geometry_cache.foreground.animation_frame = self.animation_frame;
        let map = world.map.unwrap_or(&self.empty_map);
        let assets = world.assets;
        let model = &self.elements;
        let layout = model.layout;
        let bounds = Rect::new(0., 0., layout.viewport.x as f32, layout.viewport.y as f32);
        let hidden = |name: &str| screen_model::widget_name_is_hidden(name, &model.widgets_to_hide);
        let mut list = PaintList::default();
        let mut p = ScreenPainter {
            list: &mut list,
            text: &mut fonts.text,
            images,
        };
        p.list.ops.push(PaintOp::PushClip(rect(bounds)));
        p.fill(bounds, model.background);
        for key in ordered(&model.game_widgets) {
            let widget = &model.game_widgets[&key];
            if hidden(&widget.name) || widget.rect.width <= 0. || widget.rect.height <= 0. {
                continue;
            }
            if let Some(image) = self.view_images.get(&key) {
                p.list.ops.push(PaintOp::Image {
                    image: image.image,
                    rect: rect(widget.rect),
                    uv: Uv::new(0., 0., 1., 1.),
                    sampling: Sampling::Nearest,
                    tint: [1.; 4],
                });
            } else if world.map.is_some() {
                return Err(format!("Game view '{}' has not been prepared", widget.name));
            }
        }
        let decos = ordered(&model.deco_widgets);
        for key in &decos {
            let widget = &model.deco_widgets[key];
            if widget.layer < 0 {
                p.decoration(widget, assets)?;
            }
        }
        // Preserve the legacy negative-deco/base relationship by clipping base
        // composition outside every game view. These are vector stencil clips,
        // never framebuffer hole punching or CPU/display-sized mask assets.
        let mut clips = 0;
        for key in ordered(&model.game_widgets) {
            let widget = &model.game_widgets[&key];
            if hidden(&widget.name) || widget.rect.width <= 0. || widget.rect.height <= 0. {
                continue;
            }
            let clipped = rect(widget.rect).intersection(rect(bounds));
            if clipped.width <= 0. || clipped.height <= 0. {
                continue;
            }
            let mut path = Path::default();
            rect_path(&mut path, bounds);
            rect_path(
                &mut path,
                Rect::new(clipped.x, clipped.y, clipped.width, clipped.height),
            );
            p.list
                .ops
                .push(PaintOp::PushPathClip(Arc::new(path), FillRule::EvenOdd));
            clips += 1;
        }
        p.fill(bounds, model.background);
        geometry(&mut p, &geometry_cache.base, layout, assets)?;
        for _ in 0..clips {
            p.list.ops.push(PaintOp::PopClip);
        }
        for key in decos {
            let widget = &model.deco_widgets[&key];
            if widget.layer >= 0 {
                p.decoration(widget, assets)?;
            }
        }
        for key in ordered(&model.text_widgets) {
            let widget = &model.text_widgets[&key];
            if !hidden(&widget.name) {
                p.text_widget(
                    widget,
                    map,
                    assets,
                    &self.currencies,
                    &world.time,
                    &model.ui_state,
                );
            }
        }
        for key in ordered(&model.avatar_widgets) {
            let widget = &model.avatar_widgets[&key];
            if hidden(&widget.name) {
                continue;
            }
            if let Some(content) = self.avatars.get(&key) {
                p.avatar(widget, content.source.as_deref(), content.outline)?;
            }
        }
        for key in ordered(&model.profile_widgets) {
            let widget = &model.profile_widgets[&key];
            if hidden(&widget.name) {
                continue;
            }
            if let Some(content) = self.profiles.get(&key) {
                p.profile(&content.layout, content.source.as_deref())?;
            }
        }
        for key in ordered(&model.stat_widgets) {
            let widget = &model.stat_widgets[&key];
            if !hidden(&widget.name) {
                p.stat(
                    widget,
                    assets,
                    screen_model::resolve_party_entity(map, widget.party.as_deref()),
                    self.animation_frame,
                )?;
            }
        }
        geometry(&mut p, &geometry_cache.foreground, layout, assets)?;
        for (widget, layout) in model.messages_widgets.iter().zip(&self.messages) {
            if !hidden(&widget.name) {
                p.messages(layout, assets)?;
                if let Some(command) = layout.command() {
                    if let Some(input) = self.inputs.get_mut(&ScreenTextTarget::Command(widget.id))
                    {
                        input.paint(
                            p.text,
                            p.list,
                            theframework::text_edit::EditorColors {
                                text: color(command.color),
                                ..Default::default()
                            },
                            true,
                        );
                    }
                }
            }
        }
        for key in ordered(&model.text_input_widgets) {
            let widget = &model.text_input_widgets[&key];
            if !hidden(&widget.name) {
                p.text_input_chrome(
                    widget,
                    self.focused_text() == Some(ScreenTextTarget::Input(key)),
                );
                if let Some(input) = self.inputs.get_mut(&ScreenTextTarget::Input(key)) {
                    input.paint(
                        p.text,
                        p.list,
                        theframework::text_edit::EditorColors {
                            text: color(widget.color),
                            ..Default::default()
                        },
                        true,
                    );
                } else {
                    return Err("Prepare native text inputs before painting".into());
                }
            }
        }
        for key in ordered(&model.button_widgets) {
            let widget = &model.button_widgets[&key];
            if hidden(&widget.name) {
                continue;
            }
            let entity = screen_model::resolve_party_entity(map, widget.party.as_deref());
            let command = screen_model::contextual_open_close_command(
                screen_model::resolved_widget_command(widget, assets, entity, &model.ui_state),
                self.hovered_item,
                map,
            );
            let state = command
                .as_deref()
                .map(|command| rusterix::client::rules_ui::command_state(assets, entity, command));
            let selected = model.activated_widgets.contains(&key)
                || (self.intent.is_empty() && screen_model::command_is_walk(command.as_deref()));
            let visual = screen_model::button_visual_state(
                rect(widget.rect).contains(self.cursor),
                selected,
                self.pressed_button == Some(key),
                state.as_ref(),
            );
            let content = if let Some(content) = self.buttons.get(&key) {
                ButtonContent::Image {
                    image: content.source.resolve(p.images)?,
                    source_size: [
                        content.source.image().width(),
                        content.source.image().height(),
                    ],
                    contain: content.source.fit == SourceFit::Contain,
                    inset: 4.,
                    quantity: content.quantity,
                }
            } else {
                ButtonContent::Empty
            };
            p.button(
                widget,
                assets,
                visual,
                command.as_deref(),
                self.animation_frame,
                content,
            )?;
            if self.focus == Some(ScreenFocus::Button(key)) {
                p.border(widget.rect.with_border(1.), 1., widget.label_color);
            }
        }
        let mut choices = ordered(&model.choice_widgets);
        if let Some(index) = choices
            .iter()
            .position(|id| Some(*id) == self.choice_controller.open_id())
        {
            let id = choices.remove(index);
            choices.push(id);
        }
        for id in choices {
            let widget = &model.choice_widgets[&id];
            if !hidden(&widget.name) {
                p.choice(widget, self.cursor);
                if self.focus == Some(ScreenFocus::Choice(id)) {
                    p.border(widget.rect.with_border(1.), 1., widget.indicator_color);
                }
            }
        }
        p.list.ops.push(PaintOp::PopClip);
        list.validate().map_err(str::to_string)?;
        Ok(list)
    }
}
