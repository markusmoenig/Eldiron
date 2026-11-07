//! One entry point for a definition's model, materials, icons and equipped preview.
use crate::{editor::RUSTERIX, prelude::*};
use std::{sync::Arc, time::Instant};
pub mod data;
mod preview;
use data::{Kind, Owner};
const STACK: &str = "Definition Appearance Stack";
const VIEW: &str = "Definition Appearance Preview";
const ACTIONS: &str = "Definition Appearance Actions";

fn button(id: &str, text: &str) -> Box<dyn TheWidget> {
    let mut b = TheTraybarButton::new(TheId::named(id));
    b.set_text(text.into());
    Box::new(b)
}
pub fn main_canvas(normal: TheCanvas) -> TheCanvas {
    let mut viewport = TheCanvas::new();
    let mut view = TheRenderView::new(TheId::named(VIEW));
    view.set_empty_background(MaterialCanvas);
    viewport.set_widget(view);
    let mut bar = TheHLayout::new(TheId::named("Definition Appearance Toolbar"));
    bar.limiter_mut().set_max_height(38);
    bar.set_margin(Vec4::new(8, 4, 8, 4));
    bar.set_padding(6);
    let mut title = TheText::new(TheId::named("Definition Appearance Title"));
    title.set_text("Appearance".into());
    bar.add_widget(Box::new(title));
    let mut dimension = TheGroupButton::new(TheId::named("Definition Appearance Dimension"));
    dimension.add_text("2D".into());
    dimension.add_text("3D".into());
    dimension.set_index(1);
    dimension.set_item_width(38);
    bar.add_widget(Box::new(dimension));
    let mut clip = TheDropdownMenu::new(TheId::named("Definition Appearance Clip"));
    for name in ["Idle", "Walk", "Attack"] {
        clip.add_option(name.into());
    }
    clip.limiter_mut().set_max_width(110);
    bar.add_widget(Box::new(clip));
    let mut play = TheGroupButton::new(TheId::named("Definition Appearance Play"));
    play.add_text_status_icon(String::new(), "Pause preview".into(), "mz-pause".into());
    play.add_text_status_icon(String::new(), "Play preview".into(), "mz-play".into());
    play.set_index(1);
    play.set_item_width(28);
    bar.add_widget(Box::new(play));
    let mut phase = TheTextLineEdit::new(TheId::named("Definition Appearance Phase"));
    phase.set_range(TheValue::RangeF32(0.0..=1.0));
    phase.set_value(TheValue::Float(0.));
    phase.set_status_text("Scrub the selected animation; dragging pauses playback");
    phase.limiter_mut().set_max_width(100);
    bar.add_widget(Box::new(phase));
    let mut angle = TheTextLineEdit::new(TheId::named("Definition Appearance Yaw"));
    angle.set_range(TheValue::RangeF32(0.0..=360.0));
    angle.set_value(TheValue::Float(45.));
    angle.set_status_text("Rotate the preview · slide or type an angle");
    angle.limiter_mut().set_max_width(90);
    bar.add_widget(Box::new(angle));
    bar.add_widget(button("Definition Appearance Close", "Back to map"));
    bar.set_reverse_index(Some(1));
    let mut header = TheCanvas::new();
    header.set_layout(bar);
    let mut actions = TheHLayout::new(TheId::named(ACTIONS));
    actions.limiter_mut().set_max_height(72);
    actions.set_margin(Vec4::new(8, 3, 8, 3));
    actions.set_padding(8);
    let mut top = TheCanvas::new();
    top.set_top(header);
    top.set_layout(actions);
    viewport.set_top(top);
    let mut footer = TheText::new(TheId::named("Definition Appearance Status"));
    footer.set_text("Loading appearance…".into());
    footer.limiter_mut().set_max_height(28);
    let mut status = TheHLayout::new(TheId::named("Definition Appearance Footer"));
    status.limiter_mut().set_max_height(28);
    status.set_margin(Vec4::new(8, 2, 8, 2));
    status.set_background_color(Some(MaterialCanvas));
    status.add_widget(Box::new(footer));
    let mut bottom = TheCanvas::new();
    bottom.set_layout(status);
    viewport.set_bottom(bottom);
    let mut stack = TheStackLayout::new(TheId::named(STACK));
    stack.add_canvas(normal);
    stack.add_canvas(viewport);
    let mut canvas = TheCanvas::new();
    canvas.set_layout(stack);
    canvas
}
pub fn return_button() -> Box<dyn TheWidget> {
    let mut b = TheTraybarButton::new(TheId::named("Definition Appearance Return"));
    b.set_icon_name("mz-back".into());
    b.set_status_text("Return to the character or item appearance workspace");
    b.limiter_mut().set_max_width(28);
    b.set_disabled(true);
    Box::new(b)
}
#[derive(Default)]
pub struct AppearanceWorkspace {
    owner: Option<Owner>,
    return_context: Option<ProjectContext>,
    assets: Vec<data::Asset>,
    key: Option<u64>,
    revision: u64,
    visible_last_frame: bool,
    two_d: bool,
    orbit_drag: Option<Vec2<i32>>,
    zoom: f32,
    ground: bool,
    active_state: bool,
    icon_frames: [Vec<rusterix::Texture>; 2],
    clip: usize,
    playing: bool,
    effects_active: bool,
    phase: f32,
    playhead: f32,
    yaw: f32,
    started: Option<Instant>,
    worker: Option<preview::Worker>,
    pending: bool,
    snapshot: Option<Arc<rusterix::Assets>>,
    dirty: bool,
    last_size: (u32, u32),
    last_frame: Option<Instant>,
}
impl AppearanceWorkspace {
    pub fn visible(&self, ui: &mut TheUI) -> bool {
        ui.get_stack_layout(STACK).is_some_and(|s| s.index() == 1)
    }
    pub fn wants_realtime(&self) -> bool {
        self.visible_last_frame
            && (self.playing || self.effects_active || self.pending || self.dirty)
    }
    fn show(ui: &mut TheUI, ctx: &mut TheContext, show: bool) {
        if let Some(s) = ui.get_stack_layout(STACK) {
            s.set_index(usize::from(show));
        }
        ctx.ui.relayout = true;
        ctx.ui.redraw_all = true;
    }
    pub fn reset(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        *self = Self::default();
        Self::show(ui, ctx, false);
        if let Some(w) = ui.get_widget("Definition Appearance Return") {
            w.set_disabled(true);
        }
    }
    fn invalidate(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.dirty = true;
    }
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server: &mut ServerContext,
    ) -> bool {
        if let TheEvent::Custom(id, _) = event {
            if id.name == "Open Procedural Geometry"
                || id.name == "Open Worn Geometry"
                || id.name == "Open Procedural Tile"
            {
                Self::show(ui, ctx, false);
            }
        }
        if let TheEvent::StateChanged(id, state) = event {
            if matches!(state, TheWidgetState::Selected | TheWidgetState::Clicked) {
                if matches!(
                    id.name.as_str(),
                    "Play"
                        | "Character Item Code Edit"
                        | "Character Item Data Edit"
                        | "Item Item Code Edit"
                        | "Item Item Data Edit"
                        | "Item Item"
                        | "Character Item"
                ) && self.visible(ui)
                {
                    Self::show(ui, ctx, false);
                }

                let owner = match id.name.as_str() {
                    "Character Appearance" => Some(Owner::Character(id.references)),
                    "Item Appearance" => Some(Owner::Item(id.references)),
                    _ => None,
                };
                if let Some(owner) = owner.filter(|o| o.name(project).is_some()) {
                    self.return_context = Some(
                        if project.regions.iter().any(|r| r.id == server.curr_region) {
                            ProjectContext::Region(server.curr_region)
                        } else {
                            server.pc
                        },
                    );
                    self.owner = Some(owner);
                    self.key = None;
                    self.ground = matches!(owner, Owner::Item(_))
                        && owner.data(project).is_some_and(|data| {
                            shared::ruleset_materials::worn_geometry_id(data).is_none()
                                && shared::ruleset_materials::worn_material_sources(project, data)
                                    .is_empty()
                        });
                    self.active_state = true;
                    self.yaw = 45.;
                    self.zoom = 1.;
                    self.playing = true;
                    self.phase = 0.;
                    self.playhead = 0.;
                    self.started = Some(Instant::now());
                    if let Some(w) = ui
                        .get_widget("Definition Appearance Play")
                        .and_then(|w| w.as_group_button())
                    {
                        w.set_index(1);
                    }
                    ui.set_widget_value(
                        "Definition Appearance Yaw",
                        ctx,
                        TheValue::Float(self.yaw),
                    );
                    ui.set_widget_value("Definition Appearance Phase", ctx, TheValue::Float(0.));

                    self.invalidate();
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server,
                        match owner {
                            Owner::Character(id) => ProjectContext::Character(id),
                            Owner::Item(id) => ProjectContext::Item(id),
                        },
                    );
                    crate::sidebar::Sidebar::set_navigation_page(0, ui, ctx);
                    Self::show(ui, ctx, true);
                    if let Some(w) = ui.get_widget("Definition Appearance Return") {
                        w.set_disabled(false);
                    }
                    return true;
                }
                match id.name.as_str() {
                    "Definition Appearance Bitmap" => {
                        if let Some(owner) = self.owner {
                            let source = data::body_source(owner, project);
                            if let Some(id) = project
                                .avatars
                                .values()
                                .find(|a| {
                                    a.id.to_string() == source
                                        || a.name.eq_ignore_ascii_case(&source)
                                })
                                .map(|a| a.id)
                            {
                                Self::show(ui, ctx, false);
                                set_project_context(
                                    ctx,
                                    ui,
                                    project,
                                    server,
                                    ProjectContext::Avatar(id),
                                );
                            }
                        }
                        return true;
                    }
                    "Definition Appearance Return" if self.owner.is_some() => {
                        crate::sidebar::Sidebar::set_navigation_page(0, ui, ctx);
                        Self::show(ui, ctx, true);
                        self.dirty = true;
                        return true;
                    }
                    "Definition Appearance Close" => {
                        if let Some(context) = self.return_context {
                            set_project_context(ctx, ui, project, server, context);
                        }
                        Self::show(ui, ctx, false);
                        return true;
                    }
                    "Definition Appearance Setup" => {
                        if let Some(Owner::Character(id)) = self.owner {
                            Self::show(ui, ctx, false);
                            ctx.ui.send(TheEvent::StateChanged(
                                TheId::named_with_reference("Character Item Data Edit", id),
                                TheWidgetState::Selected,
                            ));
                        }
                        return true;
                    }
                    "Definition Appearance Body" | "Definition Appearance Materials" => {
                        let kind = if id.name.ends_with("Body") {
                            Kind::Geometry
                        } else {
                            Kind::Material
                        };
                        if let Some(asset) = self.assets.iter().find(|a| a.kind == kind) {
                            Self::show(ui, ctx, false);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named(if kind == Kind::Geometry {
                                    "Open Procedural Geometry"
                                } else {
                                    "Open Procedural Tile"
                                }),
                                TheValue::Id(asset.id),
                            ));
                        }
                        return true;
                    }
                    "Definition Appearance Icon On"
                    | "Definition Appearance Icon Off"
                    | "Definition Appearance Model On"
                    | "Definition Appearance Model Off"
                    | "Definition Appearance Worn" => {
                        if let Some(Owner::Item(item_id)) = self.owner {
                            let action = match id.name.as_str() {
                                "Definition Appearance Icon On" => "Item Icon MZ On",
                                "Definition Appearance Icon Off" => "Item Icon MZ Off",
                                "Definition Appearance Model On" => "Item Geometry MZ",
                                "Definition Appearance Model Off" => "Item Geometry MZ Off",
                                _ => "Item Worn Geometry MZ",
                            };
                            self.active_state = !id.name.ends_with("Off");
                            self.two_d = id.name.contains(" Icon ");
                            self.ground = id.name != "Definition Appearance Worn";
                            for (name, index) in [
                                ("Definition Appearance Dimension", i32::from(!self.two_d)),
                                ("Definition Appearance Placement", i32::from(self.ground)),
                                ("Definition Appearance State", i32::from(!self.active_state)),
                            ] {
                                if let Some(w) =
                                    ui.get_widget(name).and_then(|w| w.as_group_button())
                                {
                                    w.set_index(index);
                                }
                            }
                            Self::show(ui, ctx, false);
                            crate::item_appearance::handle_event(
                                &TheEvent::StateChanged(
                                    TheId::named_with_reference(action, item_id),
                                    TheWidgetState::Clicked,
                                ),
                                project,
                                ctx,
                            );
                        }
                        return true;
                    }
                    _ => {}
                }
            }
        }
        if !self.visible(ui) {
            return false;
        }
        let result = (|| -> Result<bool, String> {
            match event {
                TheEvent::RenderViewClicked(id, p) if id.name == VIEW => {
                    self.orbit_drag = Some(*p);
                    return Ok(true);
                }
                TheEvent::RenderViewDragged(id, p) if id.name == VIEW => {
                    if let Some(old) = self.orbit_drag {
                        self.yaw = (self.yaw + (p.x - old.x) as f32 * 0.6).rem_euclid(360.);
                        self.orbit_drag = Some(*p);
                        self.invalidate();
                        ui.set_widget_value(
                            "Definition Appearance Yaw",
                            ctx,
                            TheValue::Float(self.yaw),
                        );
                    }
                    return Ok(true);
                }
                TheEvent::RenderViewUp(id, _) if id.name == VIEW => {
                    self.orbit_drag = None;
                    return Ok(true);
                }
                TheEvent::RenderViewScrollBy(id, d)
                | TheEvent::RenderViewPreciseScrollBy(id, d)
                    if id.name == VIEW =>
                {
                    self.zoom = (self.zoom.max(0.35) * (d.y as f32 * 0.01).exp()).clamp(0.35, 4.0);
                    self.invalidate();
                    return Ok(true);
                }
                TheEvent::RenderViewZoomBy(id, d) if id.name == VIEW => {
                    self.zoom = (self.zoom.max(0.35) / (1.0 + *d * 0.1).max(0.1)).clamp(0.35, 4.0);
                    self.invalidate();
                    return Ok(true);
                }
                TheEvent::RenderViewContext(id, _) | TheEvent::RenderViewHoverChanged(id, _)
                    if id.name == VIEW =>
                {
                    return Ok(true);
                }
                TheEvent::RenderViewDrop(id, _, _) if id.name == VIEW => {
                    return Ok(true);
                }
                TheEvent::RenderViewLostHover(id) if id.name == VIEW => {
                    return Ok(true);
                }
                TheEvent::IndexChanged(id, index)
                    if id.name == "Definition Appearance Dimension" =>
                {
                    self.two_d = *index == 0;
                    self.invalidate();
                    return Ok(true);
                }
                TheEvent::IndexChanged(id, index) if id.name == "Definition Appearance Play" => {
                    self.phase = self.playhead;
                    self.playing = *index == 1;
                    self.started = Some(Instant::now());
                    self.invalidate();
                    return Ok(true);
                }
                TheEvent::IndexChanged(id, index) if id.name == "Definition Appearance Clip" => {
                    self.clip = *index;
                    self.phase = 0.;
                    self.playhead = 0.;
                    self.started = Some(Instant::now());
                    self.invalidate();
                    return Ok(true);
                }
                TheEvent::IndexChanged(id, index)
                    if id.name == "Definition Appearance Placement" =>
                {
                    self.ground = *index == 1;
                    self.invalidate();
                    return Ok(true);
                }
                TheEvent::IndexChanged(id, index) if id.name == "Definition Appearance State" => {
                    self.active_state = *index == 0;
                    self.invalidate();
                    return Ok(true);
                }
                TheEvent::ValueChanged(id, value)
                    if id.name == "Definition Appearance Yaw"
                        || id.name == "Definition Appearance Phase" =>
                {
                    let v = match value {
                        TheValue::Float(v) | TheValue::FloatRange(v, _) => *v,
                        _ => return Ok(false),
                    };
                    if id.name.ends_with("Yaw") {
                        self.yaw = v.clamp(0., 360.);
                    } else {
                        self.phase = v.clamp(0., 1.);
                        self.playing = false;
                        if let Some(w) = ui
                            .get_widget("Definition Appearance Play")
                            .and_then(|w| w.as_group_button())
                        {
                            w.set_index(0);
                        }
                    }
                    self.invalidate();
                    return Ok(true);
                }
                _ => {}
            }
            Ok(false)
        })();
        match result {
            Ok(v) => v,
            Err(e) => {
                ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), e));
                true
            }
        }
    }
    fn refresh_form(&mut self, p: &Project, ui: &mut TheUI, ctx: &mut TheContext) {
        let Some(owner) = self.owner else {
            return;
        };
        self.assets = data::assets(owner, p);
        self.icon_frames = [Vec::new(), Vec::new()];
        if let Owner::Item(id) = owner {
            let item = &p.items[&id];
            for (state, frames) in self.icon_frames.iter_mut().enumerate() {
                let active = state == 0;
                *frames = shared::ruleset_materials::frames_for_item(p, item, active)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| item.icon_frames_for_state(active).to_vec());
            }
        }
        if let Some(actions) = ui.get_hlayout(ACTIONS) {
            actions.clear();
            match owner {
                Owner::Character(_) => {
                    actions.add_widget(button("Definition Appearance Setup", "Character setup"));
                    let source = data::body_source(owner, p);
                    if p.avatars
                        .values()
                        .any(|a| a.id.to_string() == source || a.name.eq_ignore_ascii_case(&source))
                    {
                        actions.add_widget(button("Definition Appearance Bitmap", "Edit avatar"));
                    } else if self.assets.iter().any(|a| a.kind == Kind::Geometry) {
                        actions.add_widget(button("Definition Appearance Body", "Edit model"));
                    }
                }
                Owner::Item(_) => {
                    for (id, label) in [
                        ("Definition Appearance Icon On", "Icon On"),
                        ("Definition Appearance Icon Off", "Icon Off"),
                        ("Definition Appearance Model On", "Model On"),
                        ("Definition Appearance Model Off", "Model Off"),
                    ] {
                        actions.add_widget(button(id, label));
                    }
                    if owner
                        .data(p)
                        .and_then(shared::ruleset_materials::worn_geometry_id)
                        .is_some()
                    {
                        actions.add_widget(button("Definition Appearance Worn", "Worn model"));
                    }
                    if self.assets.iter().any(|a| a.kind == Kind::Material) {
                        actions.add_widget(button("Definition Appearance Materials", "Materials"));
                    }
                    let mut placement =
                        TheGroupButton::new(TheId::named("Definition Appearance Placement"));
                    placement.add_text("On character".into());
                    placement.add_text("Object".into());
                    placement.set_item_width(105);
                    placement.set_index(i32::from(self.ground));
                    actions.add_widget(Box::new(placement));
                    let mut state =
                        TheGroupButton::new(TheId::named("Definition Appearance State"));
                    state.add_text("On".into());
                    state.add_text("Off".into());
                    state.set_item_width(38);
                    state.set_index(i32::from(!self.active_state));
                    actions.add_widget(Box::new(state));
                }
            }
        }
        ui.set_widget_value(
            "Definition Appearance Title",
            ctx,
            TheValue::Text(format!(
                "{} · Appearance",
                owner.name(p).unwrap_or("Definition")
            )),
        );
        ctx.ui.relayout = true;
    }
    pub fn update(
        &mut self,
        p: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        source_revision: u64,
    ) -> bool {
        self.visible_last_frame = self.visible(ui);
        if !self.visible_last_frame {
            return false;
        }
        let Some(owner) = self.owner else {
            return false;
        };
        if owner.name(p).is_none() {
            self.reset(ui, ctx);
            return true;
        }
        let key =
            rusterix::procedural_avatar::fingerprint(&(owner, owner.data(p), source_revision));
        if self.key != Some(key) {
            self.key = Some(key);
            let mut assets = RUSTERIX.read().unwrap().assets.clone();
            assets.rules = p.rules_source().unwrap_or_default();
            assets.item_template_ids = p.items.values().map(|i| (i.id, i.name.clone())).collect();
            assets.items = p
                .items
                .values()
                .map(|i| (i.name.clone(), (i.source.clone(), i.data.clone())))
                .collect();
            self.snapshot = Some(Arc::new(assets));
            self.refresh_form(p, ui, ctx);
            self.invalidate();
        }
        let mut redraw = false;
        if let Some(worker) = &self.worker {
            while let Ok((revision, result)) = worker.receive.try_recv() {
                self.pending = false;
                if revision == self.revision {
                    match result {
                        Ok((buffer, phase)) => {
                            self.playhead = phase;
                            if !ctx
                                .ui
                                .focus
                                .as_ref()
                                .is_some_and(|id| id.name == "Definition Appearance Phase")
                            {
                                ui.set_widget_value(
                                    "Definition Appearance Phase",
                                    ctx,
                                    TheValue::Float(phase),
                                );
                            }

                            if let Some(view) = ui.get_render_view(VIEW) {
                                *view.render_buffer_mut() = buffer;
                                view.set_needs_redraw(true);
                            }
                            ui.set_widget_value("Definition Appearance Status", ctx, TheValue::Text(
                                if matches!(self.owner, Some(Owner::Character(_))) {
                                    "Outfit from character setup · drag to turn · scroll to zoom"
                                } else { "Item preview · drag to turn · scroll to zoom" }.into()));
                            redraw = true;
                        }
                        Err(e) => {
                            redraw = true;
                            self.playing = false;
                            if let Some(w) = ui
                                .get_widget("Definition Appearance Play")
                                .and_then(|w| w.as_group_button())
                            {
                                w.set_index(0);
                            }
                            ui.set_widget_value(
                                "Definition Appearance Status",
                                ctx,
                                TheValue::Text(e),
                            );
                        }
                    }
                }
            }
        }
        if self.pending {
            return redraw;
        }
        if self.playing
            && self
                .last_frame
                .is_some_and(|t| t.elapsed().as_millis() < 33)
        {
            return redraw;
        }
        let size = ui
            .get_render_view(VIEW)
            .map(|w| {
                let d = w.dim();
                (d.width.max(1) as u32, d.height.max(1) as u32)
            })
            .unwrap_or((1, 1));
        if size != (1, 1)
            && (self.dirty || self.playing || self.effects_active || size != self.last_size)
        {
            let job = self.request(p, size, *ui.style.theme().color(MaterialCanvas));
            match job {
                Ok(job) => {
                    let worker = self.worker.get_or_insert_with(preview::Worker::new);
                    self.effects_active = if let Some(item) = &job.item {
                        rusterix::procedural_avatar::Runtime::resolve_item(item, &job.assets)
                            .is_some_and(|m| m.has_effects)
                    } else {
                        rusterix::procedural_avatar::Runtime::resolve(&job.entity, &job.assets)
                            .is_some_and(|m| m.has_entity_effects(&job.entity, &job.assets))
                    };
                    if worker.send.send(job).is_ok() {
                        self.pending = true;
                        self.dirty = false;
                        self.last_size = size;
                        self.last_frame = Some(Instant::now());
                    }
                }
                Err(e) => {
                    redraw = true;
                    self.dirty = false;
                    self.playing = false;
                    if let Some(w) = ui
                        .get_widget("Definition Appearance Play")
                        .and_then(|w| w.as_group_button())
                    {
                        w.set_index(0);
                    }
                    ui.set_widget_value("Definition Appearance Status", ctx, TheValue::Text(e));
                }
            }
        }
        redraw
    }
    fn request(
        &self,
        p: &Project,
        size: (u32, u32),
        background: [u8; 4],
    ) -> Result<preview::Request, String> {
        let owner = self.owner.ok_or("Choose an appearance")?;
        let mut entity = rusterix::Entity::default();
        let mut item = None;
        if let Owner::Character(id) = owner {
            let character = p.characters.get(&id).ok_or("Character removed")?;
            rusterix::server::data::apply_entity_data(&mut entity, &character.data);
        } else if let Owner::Item(id) = owner {
            let i = p.items.get(&id).ok_or("Item removed")?;
            let runtime = runtime_item(i, self.active_state);
            let slot = data::attribute(&i.data, "slot").unwrap_or("main_hand".into());
            entity.equipped.insert(slot, runtime.clone());
            item = Some(runtime);
        }
        let assets = self
            .snapshot
            .as_ref()
            .ok_or("Loading project assets")?
            .clone();
        entity = rusterix::server::data::character_for_display(&entity, &assets);
        let ground = matches!(owner, Owner::Item(_)) && self.ground;
        let icon = if matches!(owner, Owner::Item(_)) {
            let frames = &self.icon_frames[usize::from(!self.active_state)];
            if frames.is_empty() {
                None
            } else {
                let tick = p
                    .config
                    .parse::<toml::Table>()
                    .ok()
                    .and_then(|t| t.get("game")?.get("game_tick_ms")?.as_integer())
                    .unwrap_or(250)
                    .max(1) as f32
                    / 1000.;
                let frame = if self.playing {
                    (self.started.map_or(0., |t| t.elapsed().as_secs_f32()) / tick) as usize
                        % frames.len()
                } else {
                    ((self.phase * frames.len() as f32) as usize).min(frames.len() - 1)
                };
                Some(frames[frame].clone())
            }
        } else {
            None
        };
        let clip = ["idle", "walk", "attack"][self.clip.min(2)].to_string();
        let seconds = if self.playing {
            self.started.map_or(0., |t| t.elapsed().as_secs_f32())
        } else {
            0.
        };
        Ok(preview::Request {
            revision: self.revision,
            assets,
            entity,
            item,
            icon,
            ground,
            two_d: self.two_d,
            clip,
            seconds,
            phase: self.phase,
            playing: self.playing,
            yaw: self.yaw,
            zoom: if self.zoom == 0. { 1. } else { self.zoom },
            width: size.0,
            height: size.1,
            background,
        })
    }
}
fn runtime_item(item: &shared::item::Item, active: bool) -> rusterix::Item {
    let mut runtime = rusterix::Item::default();
    runtime.creator_id = item.id;
    rusterix::server::data::apply_item_data(&mut runtime, &item.data);
    runtime.item_type =
        data::attribute(&item.data, "ruleset_id").unwrap_or_else(|| item.name.clone());
    runtime.set_attribute("active", rusterix::Value::Bool(active));
    runtime
}

#[cfg(test)]
mod tests;
