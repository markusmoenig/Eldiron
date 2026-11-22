use super::*;
use crate::hud::HudColors;
use eldiron_presentation::ViewPick;
use rusterix::isometric::scene::SpriteOwner;
use theframework::text::{TextLayout, TextSystem};

#[path = "behavior_overview/summary.rs"]
mod summary;

enum Kind {
    Character,
    Item,
}

pub(super) struct Subject {
    kind: Kind,
    id: Uuid,
    name: String,
    template_name: Option<String>,
}

#[derive(Default)]
pub(super) struct Overview {
    content: Option<(String, Vec<summary::Section>)>,
    layout: Option<Layout>,
    bounds: Option<Bounds>,
    hover: Option<Vec2<i32>>,
    captured: bool,
    scroll: f32,
    pending: Option<(Vec2<i32>, Subject)>,
}
struct Layout {
    font: (u64, u64),
    width: f32,
    title: TextLayout,
    rows: Vec<(bool, f32, TextLayout)>,
    height: f32,
}
#[derive(Clone, Copy)]
pub(super) struct Bounds {
    panel: Rect,
    close: Rect,
    max_scroll: f32,
}

pub(super) fn available(binding: &Binding, server: &CreatorContext) -> bool {
    binding.paste.is_none()
        && binding.context.is_region()
        && !server.shared.game_mode
        && !server.shared.game_input_mode
        && !server.shared.native_game_input
        && !server.shared.text_game_mode
        && server
            .tools
            .read()
            .get_game_tool_uuid_by_command_id("tool.entity")
            == Some(binding.tool)
}

pub(super) fn pick(
    frame: &NativeWorldFrame,
    world: &NativeWorld,
    authored: &Map,
    map: &Map,
    at: Vec2<i32>,
) -> Option<Subject> {
    let size = Vec2::new(frame.binding.size.0 as f32, frame.binding.size.1 as f32);
    let at = at.map(|component| component as f32);
    let uv = (at / size).into_array();
    if uv.iter().any(|v| !(0. ..1.).contains(v)) {
        return None;
    }
    if let Some(targets) = &frame.isometric {
        return match targets.editor_subject(uv)? {
            SpriteOwner::Entity { creator_id, .. } => character(
                map.entities
                    .iter()
                    .chain(&authored.entities)
                    .find(|a| a.creator_id == creator_id)?,
            ),
            SpriteOwner::Item { creator_id, .. } => item(
                map.items
                    .iter()
                    .chain(&authored.items)
                    .find(|a| a.creator_id == creator_id)?,
            ),
            _ => None,
        };
    }
    if frame.binding.mode != EditorViewMode::D2 {
        let ViewPick::Hit(hit) = world.gpu.as_ref()?.scenes.pick_game(frame.output, uv) else {
            return None;
        };
        return match hit.geo {
            scenevm::GeoId::Character(_) => character(
                map.entities
                    .iter()
                    .enumerate()
                    .find(|(index, a)| {
                        rusterix::SceneHandler::entity_render_geo_id(a, *index) == hit.geo
                    })?
                    .1,
            ),
            scenevm::GeoId::Item(_) => item(
                map.items
                    .iter()
                    .enumerate()
                    .find(|(index, a)| {
                        rusterix::SceneHandler::item_render_geo_id(a, *index) == hit.geo
                    })?
                    .1,
            ),
            _ => None,
        };
    }

    let origin = size * 0.5 + Vec2::new(authored.offset.x, -authored.offset.y);
    let position = (at - origin) / authored.grid_size;
    let (id, is_character) = crate::tools::entity::EntityTool::pick_2d_subject(map, position)?;
    if is_character {
        character(map.entities.iter().find(|a| a.creator_id == id)?)
    } else {
        item(map.items.iter().find(|a| a.creator_id == id)?)
    }
}

fn character(entity: &rusterix::Entity) -> Option<Subject> {
    Some(Subject {
        kind: Kind::Character,
        id: entity.creator_id,
        name: entity
            .attributes
            .get_str("name")
            .unwrap_or_else(|| "Character")
            .to_owned(),
        template_name: entity.attributes.get_str("class_name").map(str::to_owned),
    })
}
fn item(item: &rusterix::Item) -> Option<Subject> {
    Some(Subject {
        kind: Kind::Item,
        id: item.creator_id,
        name: item
            .attributes
            .get_str("name")
            .unwrap_or_else(|| "Item")
            .to_owned(),
        template_name: Some(
            item.attributes
                .get_str("class_name")
                .unwrap_or(&item.item_type)
                .to_owned(),
        ),
    })
}

impl Overview {
    pub(super) fn begin_pick(&mut self, at: Vec2<i32>, subject: Subject) {
        self.pending = Some((at, subject));
    }

    pub(super) fn track_pick(&mut self, input: MapEvent) -> Option<Subject> {
        let (start, _) = self.pending.as_ref()?;
        let within_click = |at: Vec2<i32>| {
            (at - *start)
                .into_array()
                .iter()
                .all(|delta| delta.abs() <= 4)
        };
        match input {
            MapEvent::MapUp(at) if within_click(at) => {
                self.pending.take().map(|(_, subject)| subject)
            }
            MapEvent::MapUp(_) | MapEvent::MapClicked(_) | MapEvent::MapEscape => {
                self.pending = None;
                None
            }
            MapEvent::MapDragged(at) if !within_click(at) => {
                self.pending = None;
                None
            }
            _ => None,
        }
    }

    pub(super) fn cancel_pick(&mut self) {
        self.pending = None;
    }

    pub(super) fn show(&mut self, subject: Subject, project: &Project, map: Uuid) {
        let Some(region) = project.regions.iter().find(|region| region.map.id == map) else {
            return;
        };
        let sections = summary::summarize(project, region, &subject);
        let name = match subject.kind {
            Kind::Character => region
                .characters
                .get(&subject.id)
                .map(|actor| actor.name.as_str()),
            Kind::Item => region.items.get(&subject.id).map(|item| item.name.as_str()),
        }
        .filter(|name| !name.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or(subject.name);
        self.close();
        self.content = Some((name, sections));
    }

    fn close(&mut self) {
        self.content = None;
        self.layout = None;
        self.bounds = None;
        self.scroll = 0.;
    }

    pub(super) fn hover(&mut self, at: Option<Vec2<i32>>) -> bool {
        let was_close = self.close_hovered();
        self.hover = at;
        was_close != self.close_hovered()
    }
    pub(super) fn present(&mut self, bounds: Option<Bounds>) {
        self.bounds = bounds.filter(|_| self.content.is_some());
    }

    fn close_hovered(&self) -> bool {
        self.bounds
            .as_ref()
            .zip(self.hover)
            .is_some_and(|(bounds, at)| {
                bounds
                    .close
                    .contains(at.map(|component| component as f32).into_array())
            })
    }

    pub(super) fn input(&mut self, input: MapEvent) -> bool {
        match input {
            MapEvent::MapClicked(at) => {
                let point = at.map(|value| value as f32).into_array();
                self.captured = self
                    .bounds
                    .as_ref()
                    .is_some_and(|bounds| bounds.panel.contains(point));
                if self.captured && self.bounds.as_ref().unwrap().close.contains(point) {
                    self.close();
                }
                self.captured
            }
            MapEvent::MapDragged(_) => self.captured,
            MapEvent::MapUp(_) => std::mem::take(&mut self.captured),
            MapEvent::MapEscape if self.content.is_some() => {
                self.close();
                true
            }
            _ => false,
        }
    }

    pub(super) fn scroll(&mut self, pixels: f32) -> bool {
        let Some((bounds, hover)) = self.bounds.as_ref().zip(self.hover) else {
            return false;
        };
        if !bounds
            .panel
            .contains(hover.map(|value| value as f32).into_array())
        {
            return false;
        }
        self.scroll = (self.scroll + pixels).clamp(0., bounds.max_scroll);
        true
    }

    pub(super) fn paint(
        &mut self,
        binding: &Binding,
        text: &mut TextSystem,
        colors: HudColors,
        list: &mut PaintList,
    ) -> Option<Bounds> {
        let Some((name, sections)) = &self.content else {
            return None;
        };
        let width = (binding.size.0 as f32 - 24.).min(344.);
        let top = if binding.mode == EditorViewMode::Orbit {
            100.
        } else {
            43.
        };
        let available = binding.size.1 as f32 - top - 24.;
        if width < 110. || available < 74. {
            return None;
        }
        let font = text.font_stamp();
        if self
            .layout
            .as_ref()
            .is_none_or(|old| old.width != width || old.font != font)
        {
            let mut rows = Vec::new();
            let mut height = 0.;
            for section in sections {
                let heading = text.layout(&section.label, "Roboto", 13., Some(width - 35.));
                let heading_height = heading.height();
                rows.push((true, height, heading));
                height += heading_height + 5.;
                for step in &section.steps {
                    let row = text.layout(step, "Roboto", 13., Some(width - 48.));
                    let row_height = row.height();
                    rows.push((false, height, row));
                    height += row_height + 4.;
                }
                height += 9.;
            }
            if rows.is_empty() {
                let message = text.layout(
                    &fl!("behavior_overview_empty"),
                    "Roboto",
                    13.,
                    Some(width - 35.),
                );
                height = message.height() + 6.;
                rows.push((false, 0., message));
            }
            self.layout = Some(Layout {
                font,
                width,
                title: text.layout(name, "Roboto", 16., Some(width - 48.)),
                rows,
                height,
            });
        }
        let layout = self.layout.as_ref().unwrap();
        let header = (layout.title.height() + 21.).max(35.).min(available * 0.6);
        let body_height = (layout.height + 13.).min(available - header).min(490.);
        let left = binding.size.0 as f32 - width - 12.;
        let panel = Rect::new(left, top, width, header + body_height);
        let close = Rect::new(left + width - 32., top + 6., 25., 25.);
        let body = Rect::new(left + 2., top + header + 2., width - 4., body_height - 4.);
        let max_scroll = (layout.height + 8. - body.height).max(0.);
        self.scroll = self.scroll.min(max_scroll);
        let rgba = |color: [u8; 4]| color.map(|channel| channel as f32 / 255.);
        let mut border = rgba(colors.border);
        border[3] = 0.83;
        let mut fill = rgba(colors.panel);
        fill[3] = 0.88;
        list.rect(panel, 6., fill);
        list.ops.push(PaintOp::PushClip(Rect::new(
            left + 3.,
            top + 1.,
            width - 41.,
            header - 1.,
        )));
        glyphs(
            list,
            &layout.title,
            [left + 12., top + 10.],
            rgba(colors.selected_text),
        );
        list.ops.push(PaintOp::PopClip);
        if self
            .hover
            .is_some_and(|at| close.contains(at.map(|component| component as f32).into_array()))
        {
            list.rect(close, 4., rgba(colors.hover));
        }
        let cross = [close.x + 12.5, close.y + 12.5];
        for offset in [-4.5, 4.5] {
            list.ops.push(PaintOp::SampledLine {
                from: [cross[0] - 4.5, cross[1] + offset],
                to: [cross[0] + 4.5, cross[1] - offset],
                color: rgba(colors.selected_text),
            });
        }
        list.rect(
            Rect::new(left + 9., top + header, width - 18., 1.),
            0.,
            border,
        );
        list.ops.push(PaintOp::PushClip(body));
        for (heading, y, row) in &layout.rows {
            let y = body.y + 5. + y - self.scroll;
            if y + row.height() < body.y || y > body.y + body.height {
                continue;
            }
            let color = if *heading {
                rgba(colors.accent)
            } else {
                rgba(colors.selected_text)
            };
            glyphs(
                list,
                row,
                [left + if *heading { 12. } else { 21. }, y],
                color,
            );
            if !*heading {
                list.rect(Rect::new(left + 13., y + 7., 2., 2.), 1., rgba(colors.text));
            }
        }
        list.ops.push(PaintOp::PopClip);
        if max_scroll > 0. {
            let track = body.height - 4.;
            let thumb = (track * (body.height / (body.height + max_scroll)))
                .max(12.)
                .min(track);
            let y = body.y + 2. + self.scroll / max_scroll * (track - thumb);
            list.rect(Rect::new(left + width - 6., y, 3., thumb), 1., border);
        }
        Some(Bounds {
            panel,
            close,
            max_scroll,
        })
    }
}

fn glyphs(list: &mut PaintList, layout: &TextLayout, origin: [f32; 2], color: [f32; 4]) {
    for run in &layout.runs {
        list.ops.push(PaintOp::GlyphRun {
            run: run.clone(),
            origin,
            color,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Editor {
        pub(crate) fn begin_native_behavior_overview_test(
            &mut self,
            ui: &mut TheUI,
            ctx: &mut TheContext,
        ) -> Uuid {
            let mut project =
                Self::new_project(crate::project_templates::ProjectDimension::TwoDIso);
            let player = project.regions[0].characters.values().next().unwrap().id;
            project.regions[0].map.offset = Vec2::zero();
            project.regions[0].map.grid_size = 36.;
            assert!(self.open_project_as_session(project, None, ui, ctx, &mut false, &mut false));
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Set Tool"),
                TheValue::Text("tool.entity".into()),
            ));
            player
        }

        pub(crate) fn behavior_overview_pick_for_test(
            &self,
            id: Uuid,
        ) -> Option<(TheId, Vec2<i32>)> {
            let world = &self.native_viewport.world;
            let frame = world.presented.as_ref()?;
            let authored = self
                .documents
                .project
                .regions
                .iter()
                .find(|r| r.map.id == frame.binding.map)?;
            let map = self
                .play_session
                .as_ref()
                .and_then(|play| {
                    play.project()
                        .regions
                        .iter()
                        .find(|r| r.map.id == frame.binding.map)
                })
                .map_or(&authored.map, |region| &region.map);
            let position = map
                .entities
                .iter()
                .find(|actor| actor.creator_id == id)?
                .position;
            let size = Vec2::new(frame.binding.size.0 as f32, frame.binding.size.1 as f32);
            let projection = rusterix::isometric::Projection::editor(&authored.map, size);
            let feet = projection
                .screen(Vec2::new(position.x, position.z), position.y)
                .map(|value| value.round() as i32);
            for y in (feet.y - 112)..(feet.y + 10) {
                for x in (feet.x - 45)..(feet.x + 45) {
                    let at = Vec2::new(x, y);
                    if pick(frame, world, &authored.map, map, at)
                        .is_some_and(|subject| subject.id == id)
                    {
                        return Some((frame.binding.view.clone(), at));
                    }
                }
            }
            None
        }

        pub(crate) fn behavior_overview_name_for_test(&self) -> Option<&str> {
            self.native_viewport
                .world
                .behavior_overview
                .content
                .as_ref()
                .map(|(name, _)| name.as_str())
        }

        pub(crate) fn behavior_overview_waiting_for_test(&self) -> bool {
            self.native_viewport
                .world
                .behavior_overview
                .pending
                .is_some()
        }

        pub(crate) fn behavior_overview_input_for_test(&self) -> (bool, bool) {
            let world = &self.native_viewport.world;
            (
                world
                    .binding
                    .as_ref()
                    .is_some_and(|binding| available(binding, &self.server_ctx)),
                world.presented.is_some(),
            )
        }

        pub(crate) fn behavior_overview_close_for_test(&self) -> Option<(TheId, Vec2<i32>)> {
            let world = &self.native_viewport.world;
            let close = world.behavior_overview.bounds?.close;
            Some((
                world.binding.as_ref()?.view.clone(),
                Vec2::new(
                    (close.x + close.width * 0.5) as i32,
                    (close.y + close.height * 0.5) as i32,
                ),
            ))
        }
    }

    #[test]
    fn village_behavior_overview_reads_daily_routes_and_item_intents() {
        let project = shared::project_io::decode_project_streaming(
            &std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../test_projects/Village2DIso.eldiron"),
            )
            .unwrap(),
        )
        .unwrap();
        let region = &project.regions[0];
        let bram = region
            .characters
            .values()
            .find(|actor| actor.name == "Bram the Hunter")
            .unwrap();
        let sections = summary::summarize(
            &project,
            region,
            &Subject {
                kind: Kind::Character,
                id: bram.id,
                name: bram.name.clone(),
                template_name: None,
            },
        );
        assert!(sections.iter().any(|section| {
            section.label.contains("08:00")
                && section.label.contains("17:00")
                && section
                    .steps
                    .iter()
                    .any(|step| step.contains("Hunting Trail"))
        }));
        assert!(sections.iter().any(|section| {
            section.label.contains("21:18")
                && section
                    .steps
                    .iter()
                    .any(|step| step.contains("Shepherd Cot"))
        }));
        let lantern = region
            .items
            .values()
            .find(|placed| {
                project
                    .items
                    .get(&placed.item_id)
                    .is_some_and(|item| item.name == "Village Lantern")
            })
            .unwrap();
        let sections = summary::summarize(
            &project,
            region,
            &Subject {
                kind: Kind::Item,
                id: lantern.id,
                name: lantern.name.clone(),
                template_name: None,
            },
        );
        for (intent, action) in [
            ("Light", "Turn on"),
            ("Extinguish", "Turn off"),
            ("Use", "Toggle"),
        ] {
            assert!(
                sections.iter().any(|section| {
                    section.label.contains(intent)
                        && section.steps.iter().any(|step| step.contains(action))
                }),
                "{intent}"
            );
        }
    }

    #[test]
    fn behavior_overview_consumes_panel_input_only_after_presentation() {
        let mut overlay = Overview::default();
        overlay.content = Some(("A villager".into(), vec![]));
        let close = Vec2::new(195, 25);
        assert!(!overlay.input(MapEvent::MapClicked(close)));
        overlay.present(Some(Bounds {
            panel: Rect::new(10., 10., 200., 200.),
            close: Rect::new(180., 10., 30., 30.),
            max_scroll: 100.,
        }));
        overlay.hover(Some(Vec2::new(50, 50)));
        assert!(overlay.scroll(400.));
        assert_eq!(overlay.scroll, 100.);
        overlay.hover(Some(Vec2::new(300, 300)));
        assert!(!overlay.scroll(40.));
        assert!(overlay.input(MapEvent::MapClicked(close)));
        assert!(overlay.input(MapEvent::MapUp(close)));
        assert!(overlay.content.is_none());
        assert!(!overlay.input(MapEvent::MapClicked(close)));
    }
}
