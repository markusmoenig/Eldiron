//! Interactive descriptions drawn above their current world targets, in every camera.
use super::*;
use crate::world_text::{TextAnchor, WorldText};

pub(super) struct Passage {
    pub message: WorldText,
    pub map: Uuid,
    pub opened: Instant,
    pub page: usize,
    pub pages: usize,
    pub rect: Option<Rect>,
}

impl Client {
    pub fn receive_world_texts(
        &mut self,
        map: &Map,
        assets: &Assets,
        messages: Vec<(u32, WorldText)>,
    ) {
        for (receiver, mut message) in messages {
            if !Self::resolve_party_entity(map, Some("leader")).is_some_and(|e| e.id == receiver)
                || message.text.trim().is_empty()
            {
                continue;
            }
            let (sender_entity, sender_item) = match message.anchor {
                TextAnchor::Entity(id) => (Some(id), None),
                TextAnchor::Item(id) => (None, Some(id)),
                _ => (None, None),
            };
            message.text = resolver::MsgResolver::new().resolve_with_context(
                parser::MsgParser::new().parse(&message.text),
                map,
                assets,
                resolver::MessageContext {
                    sender_entity,
                    sender_item,
                    receiver_entity: Some(receiver),
                    world_time: Some(self.server_time),
                },
            );
            self.world_texts
                .retain(|p| p.map == map.id && p.message.anchor != message.anchor);
            // Bound concurrent passages so repeated inspecting cannot cover the scene.
            if self.world_texts.len() >= 4 {
                self.world_texts.remove(0);
            }
            self.world_texts.push(Passage {
                message,
                map: map.id,
                opened: Instant::now(),
                page: 0,
                pages: 1,
                rect: None,
            });
        }
    }

    pub(super) fn interact_world_text(&mut self, point: Vec2<f32>) -> bool {
        let Some(index) = self
            .world_texts
            .iter()
            .rposition(|p| p.rect.is_some_and(|r| r.contains(point)))
        else {
            return false;
        };
        let p = &mut self.world_texts[index];
        if p.page + 1 < p.pages {
            p.page += 1;
        } else {
            self.world_texts.remove(index);
        }
        true
    }

    pub(super) fn draw_world_texts(&mut self, map: &Map, assets: &Assets) {
        self.world_texts.retain(|p| {
            p.map == map.id
                && (p.pages > 1
                    || p.opened.elapsed().as_secs_f32()
                        < (p.message.text.chars().count() as f32 * 0.07 + 5.).clamp(6., 18.))
        });
        let Some(font) = self
            .messages_font
            .as_ref()
            .or_else(|| Widget::fallback_font())
        else {
            return;
        };
        let font_size = self.messages_font_size.clamp(12., 20.);
        let line_height = (font_size * 1.3).ceil();
        let mut occupied = Vec::<Rect>::new();
        for passage in &mut self.world_texts {
            passage.rect = None;
            let Some(world) = anchor_position(&passage.message.anchor, map, assets) else {
                continue;
            };
            let Some((game, point)) = self.game_widgets.values().find_map(|g| {
                project(g, world)
                    .or_else(|| {
                        carrier(&passage.message.anchor, map)
                            .filter(|e| e.is_player())
                            .map(|_| {
                                Vec2::new(
                                    g.rect.x + g.rect.width * 0.5,
                                    g.rect.y + g.rect.height * 0.55,
                                )
                            })
                    })
                    .map(|p| (g, p))
            }) else {
                continue;
            };
            let max_width = (game.rect.width - 24.).clamp(24., 320.);
            let lines = Self::wrap_tooltip_line(
                &self.draw2d,
                font,
                font_size,
                &passage.message.text,
                max_width - 32.,
            );
            let available_lines = ((game.rect.height - 44.) / line_height)
                .floor()
                .clamp(1., 4.) as usize;
            passage.pages = lines.len().div_ceil(available_lines).max(1);
            passage.page = passage.page.min(passage.pages - 1);
            let start = passage.page * available_lines;
            let end = (start + available_lines).min(lines.len());
            let page = &lines[start..end];
            let footer = if passage.pages > 1 {
                format!("{} / {}  ›", passage.page + 1, passage.pages)
            } else {
                String::new()
            };
            let text_width = page
                .iter()
                .chain((!footer.is_empty()).then_some(&footer))
                .map(|line| self.draw2d.get_text_size(font, font_size, line).0 as f32)
                .fold(0., f32::max);
            let w = (text_width + 24.).min(max_width).max(24.);
            let h = (page.len() + usize::from(passage.pages > 1)) as f32 * line_height + 12.;
            // Clamp to the game widget and keep the bottom action bar accessible.
            let bottom = self
                .button_widgets
                .values()
                .filter(|b| {
                    b.rect.y > game.rect.y + game.rect.height * 0.5
                        && !Self::widget_name_is_hidden(&b.name, &self.widgets_to_hide)
                })
                .map(|b| b.rect.y - 4.)
                .fold(game.rect.y + game.rect.height - 6., f32::min);
            let min_y = game.rect.y + 6.;
            let max_y = (bottom - h).max(min_y);
            let x = (point.x - w * 0.5).clamp(
                game.rect.x + 6.,
                (game.rect.x + game.rect.width - w - 6.).max(game.rect.x + 6.),
            );
            let mut rect = Rect::new(x, (point.y - h - 10.).clamp(min_y, max_y), w, h);
            for _ in 0..4 {
                let Some(other) = occupied.iter().find(|r| Self::rects_intersect(**r, rect)) else {
                    break;
                };
                let above = other.y - h - 6.;
                rect.y = if above >= min_y {
                    above
                } else {
                    (other.y + other.height + 6.).min(max_y)
                };
            }
            occupied.push(rect);
            passage.rect = Some(rect);
            let stride = self.target.stride();
            let safe = (
                game.rect.x as isize,
                game.rect.y as isize,
                game.rect.width as isize,
                game.rect.height as isize,
            );
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &(rect.x as isize, rect.y as isize, w as isize, h as isize),
                stride,
                &[18, 18, 16, 215],
                &safe,
            );
            for (i, line) in page.iter().enumerate() {
                self.draw2d.text_rect_blend_safe(
                    self.target.pixels_mut(),
                    &(
                        (rect.x + 8.) as isize,
                        (rect.y + 6. + i as f32 * line_height) as isize,
                        (w - 16.) as isize,
                        line_height as isize,
                    ),
                    stride,
                    font,
                    font_size,
                    line,
                    &self.messages_font_color,
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &safe,
                );
            }
            if !footer.is_empty() {
                self.draw2d.text_rect_blend_safe(
                    self.target.pixels_mut(),
                    &(
                        (rect.x + 8.) as isize,
                        (rect.y + h - line_height - 6.) as isize,
                        (w - 16.) as isize,
                        line_height as isize,
                    ),
                    stride,
                    font,
                    font_size,
                    &footer,
                    &[183, 174, 151, 255],
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &safe,
                );
            }
        }
    }
}

fn anchor_position(anchor: &TextAnchor, map: &Map, assets: &Assets) -> Option<Vec3<f32>> {
    match anchor {
        TextAnchor::Position(p) => Some(*p),
        TextAnchor::Entity(id) => map.entities.iter().find(|e| e.id == *id).map(|e| {
            e.position
                + Vec3::new(
                    0.,
                    e.attributes.get_float_default("height", 1.8).max(0.1),
                    0.,
                )
        }),
        TextAnchor::Item(id) => map
            .items
            .iter()
            .find(|i| i.id == *id)
            .map(|item| {
                if let Some(a) = crate::procedural_avatar::Runtime::resolve_item(item, assets) {
                    let s = item.attributes.get_float_default("appearance_scale", 1.);
                    let yaw = item
                        .attributes
                        .get_float_default("appearance_yaw", 0.)
                        .to_radians();
                    let x = (a.bounds.0[0] + a.bounds.1[0]) * 0.5 * s;
                    let z = (a.bounds.0[2] + a.bounds.1[2]) * 0.5 * s;
                    item.position
                        + Vec3::new(
                            x * yaw.cos() + z * yaw.sin(),
                            a.bounds.1[1] * s + 0.12,
                            -x * yaw.sin() + z * yaw.cos(),
                        )
                } else {
                    item.position
                        + Vec3::new(
                            0.,
                            item.attributes.get_float_default("height", 1.).max(0.1),
                            0.,
                        )
                }
            })
            .or_else(|| carrier(anchor, map).map(|e| e.position + Vec3::new(0., 1.8, 0.))),
        TextAnchor::Prefab(id) => map
            .block_prop_instances
            .iter()
            .find(|i| i.id == *id)
            .and_then(|i| {
                let asset = assets.block_props.get(&i.asset_id)?;
                let part = asset.parts.first()?;
                crate::block_prop_part_world_anchor(asset, i, part.id)
                    .map(|p| p + Vec3::new(0., 1., 0.))
            }),
    }
}

fn carrier<'a>(anchor: &TextAnchor, map: &'a Map) -> Option<&'a crate::Entity> {
    let TextAnchor::Item(id) = anchor else {
        return None;
    };
    map.entities
        .iter()
        .find(|e| e.get_item(*id).is_some() || e.equipped.values().any(|i| i.id == *id))
}

fn project(game: &GameWidget, world: Vec3<f32>) -> Option<Vec2<f32>> {
    let p = if game.camera.is_isometric_2d() {
        game.isometric_projection?
            .screen(Vec2::new(world.x, world.z), world.y)
    } else if Client::is_2d_camera(&game.camera) {
        game.ground_to_screen(Vec2::new(world.x, world.z)) - Vec2::new(0., world.y * game.grid_size)
    } else {
        let vp = game
            .camera_d3
            .projection_matrix(game.rect.width, game.rect.height)
            * game
                .camera_d3
                .view_matrix_for_surface(game.rect.width, game.rect.height);
        let c = vp * Vec4::new(world.x, world.y, world.z, 1.);
        if c.w <= 0. || (c.z / c.w).abs() > 1. {
            return None;
        }
        Vec2::new(
            (c.x / c.w * 0.5 + 0.5) * game.rect.width,
            (0.5 - c.y / c.w * 0.5) * game.rect.height,
        )
    };
    if p.x < 0. || p.y < 0. || p.x > game.rect.width || p.y > game.rect.height {
        return None;
    }
    Some(p + Vec2::new(game.rect.x, game.rect.y))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Client, Map, Assets) {
        let mut map = Map::default();
        let mut player = crate::Entity::new();
        player.id = 1;
        player.set_attribute("player", Value::Bool(true));
        map.entities.push(player);
        let mut item = crate::Item::new();
        item.id = 22;
        item.position = Vec3::new(5., 0., 5.);
        map.items.push(item);
        let mut c = Client::new();
        c.viewport = Vec2::new(640, 480);
        c.target = TheRGBABuffer::new(TheDim::sized(640, 480));
        let mut game = GameWidget::new();
        game.camera = PlayerCamera::D2;
        game.rect = Rect::new(0., 0., 640., 480.);
        game.grid_size = 32.;
        c.game_widgets.insert(Uuid::new_v4(), game);
        (c, map, Assets::default())
    }
    #[test]
    fn world_text_requires_explicit_private_delivery_and_follows_its_target() {
        let (mut c, mut map, assets) = fixture();
        assert!(c.world_texts.is_empty());
        let text = WorldText {
            anchor: TextAnchor::Item(22),
            text: "A weathered door.".into(),
        };
        c.receive_world_texts(&map, &assets, vec![(999, text.clone())]);
        assert!(c.world_texts.is_empty());
        c.receive_world_texts(&map, &assets, vec![(1, text)]);
        c.draw_world_texts(&map, &assets);
        let rect = c.world_texts[0].rect.unwrap();
        map.items[0].position.x += 2.;
        c.draw_world_texts(&map, &assets);
        let moved = c.world_texts[0].rect.unwrap();
        assert!(moved.x > rect.x + 50.);
        let hit = Vec2::new(moved.x + 5., moved.y + 5.);
        // The public input path must consume the click instead of issuing TerrainClicked.
        assert_eq!(
            c.touch_down(
                hit.map(|v| v as i32),
                &map,
                &assets,
                &mut SceneHandler::default()
            ),
            None
        );
        assert!(c.world_texts.is_empty());
    }
    #[test]
    fn world_text_pages_wrap_stay_in_view_and_clear_with_escape() {
        let (mut c, map, assets) = fixture();
        c.receive_world_texts(
            &map,
            &assets,
            vec![(
                1,
                WorldText {
                    anchor: TextAnchor::Item(22),
                    text: "Oak boards worn smooth by many generations of travellers. ".repeat(20),
                },
            )],
        );
        c.draw_world_texts(&map, &assets);
        let p = &c.world_texts[0];
        assert!(p.pages > 1);
        let r = p.rect.unwrap();
        assert!(r.x >= 0. && r.y >= 0. && r.x + r.width <= 640. && r.y + r.height <= 480.);
        assert!(c.interact_world_text(Vec2::new(r.x + 2., r.y + 2.)));
        assert_eq!(c.world_texts[0].page, 1);
        assert_eq!(
            c.user_event("key_down".into(), Value::Str("escape".into())),
            EntityAction::Off
        );
        assert!(c.world_texts.is_empty());
    }
    #[test]
    fn responsive_game_resize_resets_old_letterbox_offsets() {
        let (mut c, _, assets) = fixture();
        c.target_offset = Vec2::new(200, 100);
        c.screen_responsive = true;
        c.resize_surface(900, 600, &assets);
        let mut output = TheRGBABuffer::new(TheDim::sized(900, 600));
        c.insert_game_buffer(&mut output);
        assert_eq!(c.viewport, Vec2::new(900, 600));
        assert_eq!(c.target_offset, Vec2::zero());
        c.resize_surface(500, 350, &assets);
        output.resize(500, 350);
        c.insert_game_buffer(&mut output);
        assert_eq!(c.target_offset, Vec2::zero());
    }
    #[test]
    fn carried_item_look_follows_its_owner() {
        let (_, mut map, assets) = fixture();
        let item = map.items.remove(0);
        map.entities[0].inventory = vec![None; 8];
        map.entities[0].add_item(item).unwrap();
        map.entities[0].position = Vec3::new(3., 2., 4.);
        assert_eq!(
            anchor_position(&TextAnchor::Item(22), &map, &assets),
            Some(Vec3::new(3., 3.8, 4.))
        );
    }
    #[test]
    fn iso_text_uses_object_height_above_the_ground() {
        let mut game = GameWidget::new();
        game.camera = PlayerCamera::D2Iso;
        game.rect = Rect::new(0., 0., 640., 480.);
        let projection = crate::isometric::Projection {
            origin: Vec2::new(320., 240.),
            unit: 32.,
        };
        game.isometric_projection = Some(projection);
        let ground = project(&game, Vec3::zero()).unwrap();
        let top = project(&game, Vec3::new(0., 2., 0.)).unwrap();
        assert_eq!(ground.x, top.x);
        assert!(top.y < ground.y - 20.);
    }
}
