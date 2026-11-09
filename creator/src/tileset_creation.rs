//! Create tilesets from images; Import is reserved for saved Eldiron assets.
use crate::editor::{RUSTERIX, UNDOMANAGER};
use crate::prelude::*;
use shared::iso_bitmap::{self, ImportError, Kind, Settings};

#[derive(Clone)]
struct Options {
    sheet: bool,
    iso: bool,
    name: String,
    width: i32,
    frame: [i32; 2],
    advanced: bool,
    kind: i32,
    anchor: [i32; 2],
    prelit: bool,
    group: bool,
    fps: f32,
    animated: bool,
    add_all: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            sheet: true,
            iso: false,
            name: fl!("tileset_default_name"),
            width: 24,
            frame: [24, 24],
            advanced: false,
            kind: 1,
            anchor: [-1, -1],
            prelit: true,
            group: true,
            fps: 12.,
            animated: false,
            add_all: false,
        }
    }
}
impl Options {
    fn settings(&self) -> Settings {
        Settings {
            width: self.width.max(0) as u32,
            kind: match self.kind {
                1 => Kind::Ground,
                2 => Kind::Object,
                3 => Kind::Character,
                _ => Kind::Auto,
            },
            anchor: if self.anchor.iter().any(|a| *a < 0) {
                None
            } else {
                Some(self.anchor)
            },
            prelit: self.prelit,
            group_directions: self.group && !self.sheet,
            fps: self.fps,
            collection: self.name.trim().into(),
        }
    }
}
#[derive(Default)]
pub struct Creator {
    paths: Vec<std::path::PathBuf>,
    edit: Option<Uuid>,
    options: Options,
    active: bool,
}
fn number(form: &mut TheTextLayout, id: &str, label: String, value: TheValue) {
    let mut w = TheTextLineEdit::new(TheId::named(id));
    w.set_value(value);
    w.limiter_mut().set_max_width(350);
    form.add_pair(label, Box::new(w));
}
fn dropdown(form: &mut TheTextLayout, id: &str, label: String, options: &[String], selected: i32) {
    let mut w = TheDropdownMenu::new(TheId::named(id));
    for s in options {
        w.add_option(s.clone());
    }
    w.set_selected_index(selected);
    w.limiter_mut().set_max_width(350);
    form.add_pair(label, Box::new(w));
}
fn paragraph(id: &str, height: i32) -> TheTextView {
    let mut view = TheTextView::new(TheId::named(id));
    view.set_word_wrap(true);
    view.set_selectable(false);
    view.set_font_size(12.);
    view.draw_background(false);
    view.draw_border(false);
    view.limiter_mut().set_max_height(height);
    view.limiter_mut().set_max_width(350);
    view
}
impl Creator {
    fn canvas(&self) -> TheCanvas {
        let o = &self.options;
        let mut form = TheTextLayout::new(TheId::named("Tileset Creation Form"));
        form.set_margin(Vec4::new(16, 12, 16, 12));
        form.set_padding(5);
        if self.edit.is_none() {
            number(
                &mut form,
                "Tileset Name",
                fl!("name"),
                TheValue::Text(o.name.clone()),
            );
            dropdown(
                &mut form,
                "Tileset Source",
                fl!("tileset_source"),
                &[fl!("tileset_source_sheet"), fl!("tileset_source_images")],
                i32::from(!o.sheet),
            );
            dropdown(
                &mut form,
                "Tileset Layout",
                fl!("tileset_layout"),
                &[fl!("tileset_layout_2d"), fl!("tileset_layout_iso")],
                i32::from(o.iso),
            );
        }
        if self.edit.is_none() {
            dropdown(
                &mut form,
                "Tileset After Load",
                fl!("tileset_after_load"),
                &[fl!("tileset_select_in_editor"), fl!("tileset_add_all")],
                i32::from(o.add_all),
            );
        }
        if o.sheet || o.iso {
            number(
                &mut form,
                "Tileset Size",
                fl!("tileset_cell_width"),
                TheValue::Int(o.width),
            );
        }
        if o.sheet && self.edit.is_none() {
            number(
                &mut form,
                "Tileset Frame Width",
                fl!("tileset_frame_width"),
                TheValue::Int(o.frame[0]),
            );
            number(
                &mut form,
                "Tileset Frame Height",
                fl!("tileset_frame_height"),
                TheValue::Int(o.frame[1]),
            );
        }
        if self.edit.is_none() {
            let mut choose = TheTraybarButton::new(TheId::named("Tileset Choose Images"));
            choose.set_text(fl!("tileset_choose_images"));
            form.add_pair(String::new(), Box::new(choose));
            let mut info = TheText::new(TheId::named("Tileset Files"));
            info.set_text(fl!("tileset_files", count = self.paths.len().to_string()));
            form.add_pair(String::new(), Box::new(info));
        }
        if o.iso {
            let mut info = paragraph("Tileset Ground Cell", 40);
            info.set_text(fl!(
                "tileset_ground_cell",
                width = o.width.to_string(),
                height = (o.width / 2).to_string()
            ));
            info.limiter_mut().set_max_height(40);
            form.add_pair(String::new(), Box::new(info));
            let mut advanced = TheCheckButton::new(TheId::named("Tileset Advanced"));
            advanced.set_value(TheValue::Bool(o.advanced));
            form.add_pair(fl!("tileset_advanced"), Box::new(advanced));
            if o.advanced {
                dropdown(
                    &mut form,
                    "Tileset Kind",
                    fl!("tileset_kind"),
                    &[
                        fl!("tileset_kind_auto"),
                        fl!("tileset_kind_ground"),
                        fl!("tileset_kind_object"),
                        fl!("tileset_kind_character"),
                    ],
                    o.kind,
                );
                for (id, label, value) in [
                    ("Tileset Anchor X", fl!("tileset_anchor_x"), o.anchor[0]),
                    ("Tileset Anchor Y", fl!("tileset_anchor_y"), o.anchor[1]),
                ] {
                    number(&mut form, id, label, TheValue::Int(value));
                }
                if self.edit.is_some() {
                    dropdown(
                        &mut form,
                        "Tileset Lighting",
                        fl!("tileset_lighting"),
                        &[
                            fl!("tileset_lighting_original"),
                            fl!("tileset_lighting_tint"),
                        ],
                        i32::from(!o.prelit),
                    );
                    if o.animated {
                        number(
                            &mut form,
                            "Tileset FPS",
                            fl!("tileset_fps"),
                            TheValue::Float(o.fps),
                        );
                    }
                }
                if !o.sheet && self.edit.is_none() {
                    let mut group = TheCheckButton::new(TheId::named("Tileset Group"));
                    group.set_value(TheValue::Bool(o.group));
                    form.add_pair(fl!("tileset_group_directions"), Box::new(group));
                }
            }
        }
        let mut help = paragraph("Tileset Help", 50);
        help.set_text(if self.edit.is_some() {
            fl!("tileset_edit_help")
        } else {
            fl!("tileset_create_help")
        });
        help.limiter_mut().set_max_height(46);
        form.add_pair(String::new(), Box::new(help));
        let mut error = paragraph("Tileset Error", 50);
        error.limiter_mut().set_max_height(46);
        form.add_pair(String::new(), Box::new(error));
        let mut apply = TheTraybarButton::new(TheId::named("Tileset Apply"));
        apply.set_text(if self.edit.is_some() {
            fl!("apply")
        } else {
            if o.add_all {
                fl!("tileset_add_all")
            } else {
                fl!("tileset_open_source")
            }
        });
        form.add_pair(String::new(), Box::new(apply));
        let mut canvas = TheCanvas::new();
        canvas
            .limiter_mut()
            .set_max_size(Vec2::new(720, if o.iso && o.advanced { 650 } else { 480 }));
        canvas.set_layout(form);
        canvas
    }
    fn show(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        ui.show_dialog(
            &if self.edit.is_some() {
                fl!("tileset_iso_settings")
            } else {
                fl!("tileset_create")
            },
            self.canvas(),
            vec![TheDialogButtonRole::Reject],
            ctx,
        );
        ui.set_widget_value(
            TheDialogButtonRole::Reject.to_id(),
            ctx,
            TheValue::Text(fl!("project_chooser_cancel")),
        );
        ctx.ui.relayout = true;
    }
    fn read(&mut self, ui: &mut TheUI) {
        let int = |ui: &mut TheUI, name: &str, old: i32| {
            ui.get_widget(name)
                .and_then(|w| w.value().to_i32())
                .unwrap_or(old)
        };
        let boolean = |ui: &mut TheUI, name: &str, old: bool| {
            ui.get_widget(name)
                .map(|w| matches!(w.value(), TheValue::Bool(true)))
                .unwrap_or(old)
        };
        let o = &mut self.options;
        if let Some(w) = ui.get_widget("Tileset Name") {
            if let Some(name) = w.value().to_string() {
                o.name = name;
            }
        }
        o.sheet = int(ui, "Tileset Source", i32::from(!o.sheet)) == 0;
        o.iso = int(ui, "Tileset Layout", i32::from(o.iso)) == 1;
        o.add_all = int(ui, "Tileset After Load", i32::from(o.add_all)) == 1;
        o.width = int(ui, "Tileset Size", o.width);
        o.frame = [
            int(ui, "Tileset Frame Width", o.frame[0]),
            int(ui, "Tileset Frame Height", o.frame[1]),
        ];
        o.kind = int(ui, "Tileset Kind", o.kind);
        o.anchor = [
            int(ui, "Tileset Anchor X", o.anchor[0]),
            int(ui, "Tileset Anchor Y", o.anchor[1]),
        ];
        o.prelit = int(ui, "Tileset Lighting", i32::from(!o.prelit)) == 0;
        o.advanced = boolean(ui, "Tileset Advanced", o.advanced);
        o.group = boolean(ui, "Tileset Group", o.group);
        o.fps = ui
            .get_widget("Tileset FPS")
            .and_then(|w| w.value().to_f32())
            .unwrap_or(o.fps);
    }
    pub fn event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::ContextMenuSelected(_, id)
            | TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Add Tileset" =>
            {
                self.paths.clear();
                self.edit = None;
                self.options = Options::default();
                if project
                    .get_map(server)
                    .is_some_and(|map| map.camera == rusterix::MapCamera::TwoDIso)
                {
                    self.options.iso = true;
                    self.options.width = 256;
                    self.options.frame = [256, 128];
                }
                self.active = true;
                self.show(ui, ctx);
                return true;
            }
            TheEvent::Custom(id, TheValue::Id(tile)) if id.name == "Edit Iso Sprite" => {
                let Some(t) = project.tiles.get(tile) else {
                    return true;
                };
                let Some(s) = &t.isometric else {
                    return true;
                };
                self.paths.clear();
                self.edit = Some(*tile);
                self.active = true;
                self.options = Options {
                    sheet: false,
                    iso: true,
                    width: (s.pixels_per_unit * 2.).round() as i32,
                    kind: if s.ground {
                        1
                    } else if t.role == rusterix::TileRole::Character {
                        3
                    } else {
                        2
                    },
                    anchor: s.anchor,
                    prelit: s.prelit,
                    fps: s.frames_per_second,
                    animated: s.frames_per_direction > 1,
                    advanced: true,
                    ..Default::default()
                };
                self.show(ui, ctx);
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Tileset Choose Images" && self.active =>
            {
                self.read(ui);
                ctx.ui.open_file_requester(
                    TheId::named("Tileset Creation Images"),
                    fl!("tileset_choose_images"),
                    TheFileExtension::new(fl!("png_image"), vec!["png".into(), "PNG".into()]),
                );
                return true;
            }
            TheEvent::FileRequesterResult(id, paths) if id.name == "Tileset Creation Images" => {
                if !paths.is_empty() {
                    self.paths = paths.clone();
                    if self.options.name == fl!("tileset_default_name")
                        && let Some(name) = paths[0].file_stem()
                    {
                        self.options.name = name.to_string_lossy().into_owned();
                    }
                    self.show(ui, ctx);
                }
                return true;
            }
            TheEvent::IndexChanged(id, _)
                if self.active
                    && matches!(
                        id.name.as_str(),
                        "Tileset Source" | "Tileset Layout" | "Tileset After Load"
                    ) =>
            {
                let old = self.options.clone();
                self.read(ui);
                let o = &mut self.options;
                if old.iso != o.iso {
                    if o.width == if old.iso { 256 } else { 24 } {
                        o.width = if o.iso { 256 } else { 24 };
                    }
                    if old.frame == [old.width, if old.iso { old.width / 2 } else { old.width }] {
                        o.frame = [o.width, if o.iso { o.width / 2 } else { o.width }];
                    }
                }
                if old.sheet != o.sheet {
                    o.kind = if o.sheet { 1 } else { 0 };
                }
                self.show(ui, ctx);
                return true;
            }
            TheEvent::ValueChanged(id, _)
                if self.active
                    && matches!(id.name.as_str(), "Tileset Advanced" | "Tileset Size") =>
            {
                let old = self.options.clone();
                self.read(ui);
                if id.name == "Tileset Size"
                    && old.frame == [old.width, if old.iso { old.width / 2 } else { old.width }]
                {
                    self.options.frame = [
                        self.options.width,
                        if self.options.iso {
                            self.options.width / 2
                        } else {
                            self.options.width
                        },
                    ];
                }
                self.show(ui, ctx);
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Tileset Apply" && self.active =>
            {
                self.read(ui);
                let result = self.apply(project, ui, ctx, server);
                match result {
                    Ok(()) => {
                        ui.clear_dialog();
                        self.active = false;
                        self.paths.clear();
                        self.edit = None;
                    }
                    Err(error) => {
                        if let Some(w) = ui.get_widget("Tileset Error") {
                            w.set_value(TheValue::Text(error));
                        }
                        ctx.ui.redraw_all = true;
                    }
                }
                return true;
            }
            _ => {}
        }
        false
    }
    fn apply(
        &self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server: &mut ServerContext,
    ) -> Result<(), String> {
        let settings = self.options.settings();
        if let Some(id) = self.edit {
            settings.validate().map_err(localize_error)?;
            let before = project
                .tiles
                .get(&id)
                .ok_or_else(|| fl!("tileset_error_removed"))?
                .clone();
            let mut after = before.clone();
            let old = after
                .isometric
                .as_ref()
                .ok_or_else(|| fl!("tileset_error_removed"))?;
            let texture = after
                .textures
                .first()
                .ok_or_else(|| fl!("tileset_error_empty"))?;
            let mut layout = settings.layout(texture, settings.kind == Kind::Ground);
            layout.directions = old.directions.max(1);
            layout.frames_per_direction = old.frames_per_direction.max(1);
            layout.walk_frames = old.walk_frames;
            layout.frames_per_second = settings.fps;
            after.isometric = Some(layout);
            let atom = ProjectUndoAtom::TileEdit(before, after);
            atom.redo(project, ui, ctx, server);
            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
        } else {
            if settings.collection.is_empty() {
                return Err(fl!("tileset_error_name"));
            }
            if self.paths.is_empty() {
                return Err(fl!("tileset_error_empty"));
            }
            if self.paths.len() > 2048 {
                return Err(fl!("tileset_error_many"));
            }
            let mut images = Vec::new();
            for path in &self.paths {
                let bytes = std::fs::read(path)
                    .map_err(|_| fl!("tileset_error_read", name = path.display().to_string()))?;
                let image = crate::editor::Editor::decode_png_tile(bytes)
                    .ok_or_else(|| fl!("tileset_error_png", name = path.display().to_string()))?;
                let image = &image.buffer[0];
                images.push((
                    path.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    rusterix::Texture::new(
                        image.pixels().to_vec(),
                        image.dim().width as usize,
                        image.dim().height as usize,
                    ),
                ));
            }
            let mut source = create_source(images, &self.options)?;
            let mut collection = None;
            let mut tiles = Vec::new();
            if self.options.add_all {
                let grid = source.grid_dimensions();
                let selected = TheRGBARegionSequence {
                    regions: (0..source.buffer.dim().height / grid.y)
                        .flat_map(|y| {
                            (0..source.buffer.dim().width / grid.x).map(move |x| {
                                TheRGBARegion::new(
                                    (x * grid.x) as usize,
                                    (y * grid.y) as usize,
                                    grid.x as usize,
                                    grid.y as usize,
                                )
                            })
                        })
                        .collect(),
                };
                let id = source.id;
                let mut staged = Project::default();
                staged.add_tilemap(source);
                let ids = add_selected(
                    &mut staged,
                    id,
                    &selected,
                    false,
                    rusterix::TileRole::Nature,
                )?;
                tiles = ids
                    .iter()
                    .filter_map(|id| staged.tiles.shift_remove(id))
                    .collect();
                collection = staged.tile_collections.shift_remove(&id);
                source = staged.tilemaps.remove(0);
            }
            let source_id = source.id;
            let previous_context = server.pc;
            let atom = ProjectUndoAtom::CreateTileSource(Box::new(source), collection, tiles);
            atom.redo(project, ui, ctx, server);
            if self.options.add_all {
                crate::undo::project_helper::set_project_context(
                    ctx,
                    ui,
                    project,
                    server,
                    previous_context,
                );
            }
            UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
            if self.options.add_all {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Reveal Tile Collection"),
                    TheValue::Id(source_id),
                ));
            }
        }
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tiles"),
            TheValue::Empty,
        ));
        RUSTERIX.write().unwrap().set_dirty();
        ctx.ui.redraw_all = true;
        Ok(())
    }
}
fn localize_error(error: ImportError) -> String {
    match error {
        ImportError::Size => fl!("tileset_error_size"),
        ImportError::Rate => fl!("tileset_error_rate"),
        ImportError::Empty => fl!("tileset_error_empty"),
        ImportError::TooMany => fl!("tileset_error_many"),
        ImportError::Image(name) => fl!("tileset_error_image", name = name),
        ImportError::Duplicate(name) => fl!("tileset_error_duplicate", name = name),
        ImportError::Views(name) => fl!("tileset_error_views", name = name),
        ImportError::Canvas(name) => fl!("tileset_error_canvas", name = name),
        ImportError::Frame {
            actor,
            clip,
            frame,
            direction,
        } => fl!(
            "tileset_error_frame",
            name = actor,
            clip = clip,
            frame = frame.to_string(),
            direction = direction.to_string()
        ),
        ImportError::Direction { actor, clip } => {
            fl!("tileset_error_direction", name = actor, clip = clip)
        }
    }
}
/// Loading a source never publishes tiles. The dock owns selection and animation.
fn create_source(
    mut images: Vec<(String, rusterix::Texture)>,
    o: &Options,
) -> Result<shared::tilemap::Tilemap, String> {
    if o.name.trim().is_empty() {
        return Err(fl!("tileset_error_name"));
    }
    if images.is_empty() {
        return Err(fl!("tileset_error_empty"));
    }
    if images.len() > 2048 {
        return Err(fl!("tileset_error_many"));
    }
    if o.iso {
        o.settings().validate().map_err(localize_error)?;
    }
    let mut source = shared::tilemap::Tilemap::new();
    source.name = o.name.trim().into();
    source.isometric = o.iso.then(|| o.settings());
    if o.sheet {
        if images.len() != 1 {
            return Err(fl!("tileset_error_one_sheet"));
        }
        let (_, sheet) = images.remove(0);
        let [w, h] = o.frame;
        if w < 1 || h < 1 || w > 4096 || h > 4096 {
            return Err(fl!("tileset_error_frame_size"));
        }
        if sheet.width % w as usize != 0 || sheet.height % h as usize != 0 {
            return Err(fl!("tileset_error_sheet_size"));
        }
        source.grid_size = w;
        source.frame_size = (w != h || o.iso).then_some([w, h]);
        source.buffer = TheRGBABuffer::from(sheet.data, sheet.width as u32, sheet.height as u32);
    } else {
        for (name, t) in &images {
            if t.width == 0 || t.height == 0 || t.width > 4096 || t.height > 4096 {
                return Err(fl!("tileset_error_image", name = name.clone()));
            }
        }
        let w = images.iter().map(|(_, t)| t.width).max().unwrap();
        let h = images.iter().map(|(_, t)| t.height).max().unwrap();
        let columns = images.len().min(12);
        let rows = images.len().div_ceil(columns);
        let atlas_w = columns * w;
        let atlas_h = rows * h;
        if atlas_w
            .checked_mul(atlas_h)
            .and_then(|n| n.checked_mul(4))
            .is_none_or(|n| n > 512 * 1024 * 1024)
        {
            return Err(fl!("tileset_error_source_size"));
        }
        source.grid_size = w as i32;
        source.frame_size = Some([w as i32, h as i32]);
        source.buffer = TheRGBABuffer::new(TheDim::sized(atlas_w as i32, atlas_h as i32));
        for (index, (name, image)) in images.into_iter().enumerate() {
            let (x, y) = ((index % columns) * w, (index / columns) * h);
            for row in 0..image.height {
                let at = ((y + row) * atlas_w + x) * 4;
                source.buffer.pixels_mut()[at..at + image.width * 4].copy_from_slice(
                    &image.data[row * image.width * 4..(row + 1) * image.width * 4],
                );
            }
            source.source_frames.push(shared::tilemap::SourceFrame {
                name,
                region: TheRGBARegion::new(x, y, image.width, image.height),
            });
        }
    }
    Ok(source)
}
/// Both square sheets and iso/image sources use the same explicit selection step.
pub(crate) fn add_selected(
    project: &mut Project,
    source_id: Uuid,
    sequence: &TheRGBARegionSequence,
    animation: bool,
    role: rusterix::TileRole,
) -> Result<Vec<Uuid>, String> {
    let source = project
        .get_tilemap(source_id)
        .ok_or_else(|| fl!("tileset_error_removed"))?;
    let include_empty = animation
        || source
            .isometric
            .as_ref()
            .is_some_and(|s| s.group_directions && !source.source_frames.is_empty());
    let images = source.selected_images(sequence, include_empty);
    if images.is_empty() {
        return Err(fl!("tileset_error_transparent"));
    }
    let source_name = source.name.clone();
    let iso = source.isometric.clone();
    let source_kind = iso.as_ref().map(|s| s.kind);
    let originals = source
        .source_frames
        .iter()
        .map(|f| ((f.region.x, f.region.y), f.name.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let mut regions_by_name = std::collections::BTreeMap::<String, Vec<TheRGBARegion>>::new();
    for region in &sequence.regions {
        let name = originals
            .get(&(region.x, region.y))
            .cloned()
            .unwrap_or_else(|| format!("{} ({}, {})", source.name, region.x, region.y));
        regions_by_name
            .entry(name)
            .or_default()
            .push(region.clone());
    }
    let mut temporary = Project::default();
    let ids = if animation {
        let textures = images.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>();
        if textures
            .iter()
            .any(|t| t.width != textures[0].width || t.height != textures[0].height)
        {
            return Err(fl!("tileset_error_canvas", name = source_name));
        }
        let mut tile = rusterix::Tile::from_textures(textures);
        tile.alias = source_name.clone();
        tile.role = role;
        if let Some(settings) = &iso {
            let mut layout = settings.layout(&tile.textures[0], settings.kind == Kind::Ground);
            layout.frames_per_direction = tile.textures.len();
            tile.isometric = Some(layout);
        }
        let id = tile.id;
        temporary.tiles.insert(id, tile);
        vec![id]
    } else if let Some(settings) = &iso {
        iso_bitmap::import(&mut temporary, images.clone(), settings).map_err(localize_error)?
    } else {
        images
            .iter()
            .map(|(name, texture)| {
                let mut texture = texture.clone();
                texture.generate_normals(true);
                let mut tile = rusterix::Tile::from_texture(texture);
                tile.alias = name.clone();
                tile.role = role;
                let id = tile.id;
                temporary.tiles.insert(id, tile);
                id
            })
            .collect::<Vec<_>>()
    };
    if !project.tile_collections.contains_key(&source_id) {
        let mut c = shared::project::TileCollectionAsset::new(source_name);
        c.id = source_id;
        project.add_tile_collection(c);
    }
    for id in &ids {
        let mut tile = temporary.tiles.shift_remove(id).unwrap();
        if iso.is_none() || source_kind != Some(Kind::Auto) {
            tile.role = role;
        }
        let matching = if animation {
            sequence.regions.clone()
        } else if let Some(regions) = regions_by_name.get(&tile.alias) {
            regions.clone()
        } else {
            regions_by_name
                .iter()
                .filter(|(name, _)| {
                    if let Some((actor, clip)) = tile.alias.split_once(" / ") {
                        name.starts_with(&format!("{actor}_"))
                            && name
                                .rsplit('_')
                                .next()
                                .is_some_and(|s| s.to_ascii_lowercase().starts_with(clip))
                    } else {
                        name.starts_with(&format!("{}_", tile.alias))
                    }
                })
                .flat_map(|(_, regions)| regions.iter().cloned())
                .collect()
        };
        let local = shared::tilemap::Tile {
            id: *id,
            name: tile.alias.clone(),
            role: tile.role,
            sequence: TheRGBARegionSequence { regions: matching },
            blocking: tile.blocking,
            ..Default::default()
        };
        project
            .get_tilemap_mut(source_id)
            .unwrap()
            .tiles
            .push(local);
        let entry = rusterix::TileSource::SingleTile(*id);
        let position = project.tile_collections[&source_id].entries.len();
        project.add_source_to_collection(&source_id, entry);
        project.set_collection_tile_board_position(
            &source_id,
            entry,
            Vec2::new((position % 12) as i32, (position / 12) as i32),
        );
        project.tiles.insert(*id, tile);
    }
    Ok(ids)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn image(w: usize, h: usize, color: [u8; 4]) -> rusterix::Texture {
        rusterix::Texture::new(color.repeat(w * h), w, h)
    }
    fn selection(source: &shared::tilemap::Tilemap, cells: &[(i32, i32)]) -> TheRGBARegionSequence {
        let g = source.grid_dimensions();
        TheRGBARegionSequence {
            regions: cells
                .iter()
                .map(|(x, y)| {
                    TheRGBARegion::new(
                        (x * g.x) as usize,
                        (y * g.y) as usize,
                        g.x as usize,
                        g.y as usize,
                    )
                })
                .collect(),
        }
    }
    #[test]
    fn loading_source_waits_for_selection_and_can_publish_static_or_animation() {
        let o = Options {
            name: "Ground".into(),
            width: 2,
            frame: [2, 2],
            ..Default::default()
        };
        let mut source =
            create_source(vec![("sheet".into(), image(4, 4, [1, 2, 3, 255]))], &o).unwrap();
        for y in 2..4 {
            for x in 2..4 {
                source.buffer.pixels_mut()[(y * 4 + x) * 4 + 3] = 0;
            }
        }
        let id = source.id;
        let one = selection(&source, &[(0, 0)]);
        let anim = selection(&source, &[(0, 1), (1, 1)]);
        let mut p = Project::default();
        let before = p.tiles.len();
        p.add_tilemap(source);
        assert_eq!(p.tiles.len(), before);
        assert!(p.tile_collections.is_empty());
        let static_ids = add_selected(&mut p, id, &one, false, rusterix::TileRole::Nature).unwrap();
        assert_eq!(static_ids.len(), 1);
        assert_eq!(p.tiles.len(), before + 1);
        let animation_ids =
            add_selected(&mut p, id, &anim, true, rusterix::TileRole::Nature).unwrap();
        assert_eq!(animation_ids.len(), 1);
        assert_eq!(p.tiles[&animation_ids[0]].textures.len(), 2);
        assert!(
            p.tiles[&animation_ids[0]].textures[1]
                .data
                .chunks_exact(4)
                .all(|p| p[3] == 0),
            "Explicit animation selection preserves empty frames"
        );
        assert_eq!(p.tile_collections[&id].entries.len(), 2);
        assert_eq!(p.get_tilemap(id).unwrap().tiles.len(), 2);
    }
    #[test]
    fn rectangular_iso_sheet_and_original_png_canvases_survive_selection() {
        for sheet in [true, false] {
            let o = Options {
                name: "Walls".into(),
                sheet,
                iso: true,
                width: 4,
                frame: [4, 8],
                ..Default::default()
            };
            let images = if sheet {
                vec![("walls".into(), image(8, 8, [1, 2, 3, 255]))]
            } else {
                vec![
                    ("wall".into(), image(4, 8, [1, 2, 3, 255])),
                    ("rock".into(), image(2, 4, [4, 5, 6, 255])),
                ]
            };
            let source = create_source(images, &o).unwrap();
            let id = source.id;
            let selected = selection(&source, &[(0, 0), (1, 0)]);
            let mut p = Project::default();
            p.add_tilemap(source);
            let ids =
                add_selected(&mut p, id, &selected, false, rusterix::TileRole::ManMade).unwrap();
            assert_eq!(ids.len(), 2);
            let wall = ids
                .iter()
                .map(|id| &p.tiles[id])
                .find(|tile| tile.alias.starts_with("Walls") || tile.alias == "wall")
                .unwrap();
            assert_eq!(wall.textures[0].height, 8);
            assert_eq!(wall.isometric.as_ref().unwrap().anchor, [2, 7]);
            if !sheet {
                let rock = ids
                    .iter()
                    .map(|id| &p.tiles[id])
                    .find(|t| t.alias == "rock")
                    .unwrap();
                assert_eq!((rock.textures[0].width, rock.textures[0].height), (2, 4));
            }
            let restored = shared::project_io::decode_project(
                &shared::project_io::encode_project(&p).unwrap(),
            )
            .unwrap();
            assert_eq!(p.tilemaps, restored.tilemaps);
        }
    }
    #[test]
    fn selecting_incomplete_directions_is_atomic_and_animation_remains_one_tile() {
        let o = Options {
            name: "Walls".into(),
            sheet: false,
            iso: true,
            width: 4,
            kind: 2,
            ..Default::default()
        };
        let source = create_source(
            ["E", "N", "W", "S"]
                .iter()
                .map(|d| (format!("wall_{d}"), image(4, 8, [255; 4])))
                .collect(),
            &o,
        )
        .unwrap();
        let id = source.id;
        let part = selection(&source, &[(0, 0), (1, 0)]);
        let all = selection(&source, &[(0, 0), (1, 0), (2, 0), (3, 0)]);
        let mut p = Project::default();
        p.add_tilemap(source);
        let count = p.tiles.len();
        assert!(add_selected(&mut p, id, &part, false, rusterix::TileRole::ManMade).is_err());
        assert_eq!(p.tiles.len(), count);
        assert!(p.tile_collections.is_empty());
        let ids = add_selected(&mut p, id, &all, false, rusterix::TileRole::ManMade).unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(p.tiles[&ids[0]].isometric.as_ref().unwrap().directions, 4);
        let animated = add_selected(&mut p, id, &part, true, rusterix::TileRole::ManMade).unwrap();
        assert_eq!(animated.len(), 1);
        assert_eq!(p.tiles[&animated[0]].textures.len(), 2);
        assert_eq!(
            p.tiles[&animated[0]]
                .isometric
                .as_ref()
                .unwrap()
                .walk_frames,
            0
        );
    }
    #[test]
    fn all_at_once_undo_restores_source_and_picker_together() {
        let o = Options {
            name: "Source".into(),
            frame: [2, 2],
            width: 2,
            ..Default::default()
        };
        let source =
            create_source(vec![("sheet".into(), image(4, 2, [1, 2, 3, 255]))], &o).unwrap();
        let id = source.id;
        let selected = selection(&source, &[(0, 0), (1, 0)]);
        let mut staged = Project::default();
        staged.add_tilemap(source);
        let ids = add_selected(
            &mut staged,
            id,
            &selected,
            false,
            rusterix::TileRole::Nature,
        )
        .unwrap();
        let atom = ProjectUndoAtom::CreateTileSource(
            Box::new(staged.tilemaps.remove(0)),
            staged.tile_collections.shift_remove(&id),
            ids.iter().map(|id| staged.tiles[id].clone()).collect(),
        );
        let mut project = Project::default();
        let before = project.tiles.len();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(800, 600, 1.);
        ui.init(&mut ctx);
        let mut server = ServerContext::default();
        atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.tiles.len(), before + 2);
        assert!(project.get_tilemap(id).is_some());
        assert_eq!(project.tile_collections[&id].entries.len(), 2);
        atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.tiles.len(), before);
        assert!(project.get_tilemap(id).is_none());
        assert!(!project.tile_collections.contains_key(&id));
        atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.tiles.len(), before + 2);
    }
    #[test]
    fn creation_leaves_lighting_and_animation_speed_in_sprite_settings() {
        let mut creator = Creator::default();
        creator.options.iso = true;
        creator.options.advanced = true;
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1000, 900, 1.);
        ui.init(&mut ctx);
        creator.show(&mut ui, &mut ctx);
        assert!(ui.get_widget("Tileset Lighting").is_none());
        assert!(ui.get_widget("Tileset FPS").is_none());
        assert!(ui.get_widget("Tileset After Load").is_some());
        let mut project = Project::default();
        let mut tile = rusterix::Tile::from_textures(vec![image(2, 4, [255; 4]); 2]);
        tile.isometric = Some(rusterix::isometric::SpriteLayout {
            bitmap: true,
            prelit: true,
            pixels_per_unit: 2.,
            directions: 1,
            frames_per_direction: 2,
            frames_per_second: 12.,
            ..Default::default()
        });
        let id = tile.id;
        project.tiles.insert(id, tile);
        creator.event(
            &TheEvent::Custom(TheId::named("Edit Iso Sprite"), TheValue::Id(id)),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut ServerContext::default(),
        );
        assert!(ui.get_widget("Tileset Lighting").is_some());
        assert!(ui.get_widget("Tileset FPS").is_some());
        assert!(ui.get_widget("Tileset After Load").is_none());
    }
    #[test]
    fn tileset_and_recent_chooser_messages_exist_in_every_locale_with_matching_variables() {
        let en = include_str!("../i18n/en/eldiron_creator.ftl");
        let messages = |text: &str| {
            text.lines()
                .filter_map(|line| line.split_once(" = "))
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        let en = messages(en);
        let variable = regex::Regex::new(r"\{\s*\$([a-zA-Z_0-9]+)\s*\}").unwrap();
        let vars = |s: &str| {
            variable
                .captures_iter(s)
                .map(|c| c[1].to_string())
                .collect::<std::collections::BTreeSet<_>>()
        };
        for (locale, text) in [
            ("de", include_str!("../i18n/de/eldiron_creator.ftl")),
            ("es", include_str!("../i18n/es/eldiron_creator.ftl")),
            ("ru", include_str!("../i18n/ru/eldiron_creator.ftl")),
            ("zh-CN", include_str!("../i18n/zh-CN/eldiron_creator.ftl")),
            ("zh-TW", include_str!("../i18n/zh-TW/eldiron_creator.ftl")),
        ] {
            let translated = messages(text);
            for (key, value) in en.iter().filter(|(k, _)| {
                k.starts_with("tileset_")
                    || k.starts_with("project_")
                    || k.starts_with("example_")
                    || k.starts_with("status_example_")
                    || k.as_str() == "status_project_choose"
            }) {
                let translation = translated
                    .get(key)
                    .unwrap_or_else(|| panic!("{locale} missing {key}"));
                assert!(!translation.trim().is_empty());
                assert_eq!(vars(value), vars(translation), "{locale} {key} variables");
            }
        }
    }
    #[test]
    fn creation_dialog_layout_review() {
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1000, 900, 1.);
        ui.init(&mut ctx);
        ui.set_theme(Box::new(TheBrownTheme::new()), &mut ctx);
        ui.canvas.root = true;
        ui.canvas.set_dim(TheDim::sized(1000, 900), &mut ctx);
        let mut creator = Creator::default();
        creator.options.iso = true;
        creator.options.width = 256;
        creator.options.frame = [256, 512];
        creator.options.advanced = true;
        creator.show(&mut ui, &mut ctx);
        for id in [
            "Tileset Source",
            "Tileset Layout",
            "Tileset Choose Images",
            "Tileset Frame Height",
            "Tileset Anchor Y",
            "Tileset Apply",
        ] {
            assert!(ui.get_widget(id).is_some(), "Missing {id}");
        }
        let mut pixels = vec![0; 1000 * 900 * 4];
        ui.draw(&mut pixels, &mut ctx);
        if let Ok(path) = std::env::var("ELDIRON_TILESET_REVIEW") {
            std::fs::write(
                path,
                TheRGBABuffer::from(pixels, 1000, 900).to_png().unwrap(),
            )
            .unwrap();
        }
    }
}
