//! Palette replacement is previewed, then committed as one project edit.
use crate::{editor::UNDOMANAGER, prelude::*, undo::project_atoms::ProjectUndoAtom};
use shared::palette::{PaletteState, PaletteSwitch};
mod lospec;
const DIALOG: &str = "Switch Project Palette";
#[derive(Default)]
pub struct PaletteSwitchController {
    pending: Option<PaletteSwitch>,
    receiver: Option<std::sync::mpsc::Receiver<Result<lospec::Palette, String>>>,
    source_label: Option<String>,
}

pub(crate) fn ramps(project: &Project) -> Vec<Vec<u16>> {
    // Default material families plus authored Color Ramp stops in every MZ asset.
    let mut result = vec![];
    for bundle in project.procedural_materials.values() {
        if let Ok(asset) = denrim_materials::integration::EditableMaterialAsset::reopen(bundle) {
            for record in asset.instances {
                let mut grouped =
                    std::collections::BTreeMap::<(String, String), Vec<(String, u16)>>::new();
                for binding in record.palette_bindings {
                    if binding.key.parameter.starts_with("color")
                        && binding.key.parameter != "color"
                    {
                        grouped
                            .entry((binding.key.asset, binding.key.tile))
                            .or_default()
                            .push((binding.key.parameter, binding.index as u16));
                    }
                }
                for mut group in grouped.into_values() {
                    group.sort_by(|a, b| a.0.cmp(&b.0));
                    if group.len() > 1 {
                        result.push(group.into_iter().map(|(_, i)| i).collect());
                    }
                }
            }
        }
    }
    fn effect_ramps(doc: &denrim_materials::geometry::Document, out: &mut Vec<Vec<u16>>) {
        use denrim_materials::geometry::Operation;
        for tile in doc.rows.iter().flat_map(|r| &r.tiles) {
            match &tile.op {
                Operation::ParticleEffect { effect } => out.push(effect.palette.to_vec()),
                Operation::Group { document, .. } | Operation::Asset { document, .. } => {
                    effect_ramps(document, out)
                }
                _ => {}
            }
        }
    }
    for source in project.procedural_appearances.values() {
        if let Ok(doc) = denrim_materials::geometry::Document::load(source) {
            effect_ramps(&doc, &mut result);
        }
    }
    result.extend([
        vec![1, 2, 3, 4, 5, 6, 7, 9, 8],
        vec![17, 18, 19, 20, 21, 22, 23],
        vec![38, 37, 36, 35, 34, 33],
        vec![39, 40, 41, 42, 43, 44, 45],
    ]);
    result
}
impl PaletteSwitchController {
    pub fn reset(&mut self) {
        self.pending = None;
        self.receiver = None;
        self.source_label = None;
    }
    pub fn update(&mut self, ctx: &mut TheContext) -> bool {
        let result = self.receiver.as_ref().and_then(|r| r.try_recv().ok());
        if let Some(result) = result {
            self.receiver = None;
            match result {
                Ok(palette) => {
                    self.source_label = Some(if palette.author.trim().is_empty() {
                        format!("{} · Lospec", palette.name)
                    } else {
                        format!("{} · {} · Lospec", palette.name, palette.author)
                    });
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Preview Palette Switch"),
                        TheValue::Text(palette.colors.join("\n")),
                    ));
                }
                Err(e) => ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), e)),
            }
            return true;
        }
        false
    }
    fn lospec_dialog(ui: &mut TheUI, ctx: &mut TheContext) {
        let mut canvas = TheCanvas::new();
        canvas.limiter_mut().set_max_size(Vec2::new(520, 125));
        let mut layout = TheVLayout::new(TheId::empty());
        layout.set_margin(Vec4::new(12, 10, 12, 10));
        let mut label = TheText::new(TheId::empty());
        label.set_text("Lospec palette name or URL".into());
        layout.add_widget(Box::new(label));
        let mut name = TheTextLineEdit::new(TheId::named("Dialog Value"));
        name.set_info_text(Some("Endesga 32, Sweetie 16, or a Lospec URL".into()));
        layout.add_widget(Box::new(name));
        canvas.set_layout(layout);
        ui.show_dialog(
            "Load Lospec Palette",
            canvas,
            vec![TheDialogButtonRole::Accept, TheDialogButtonRole::Reject],
            ctx,
        );
        ctx.ui.set_focus(&TheId::named("Dialog Value"));
    }
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server: &mut ServerContext,
    ) -> bool {
        if let TheEvent::ValueChanged(id, _) = event {
            if id.name == "Dialog Value" && ui.dialog_text == "Load Lospec Palette" {
                ctx.ui.send(TheEvent::StateChanged(
                    TheId::named("TheDialogButtonRole::Accept"),
                    TheWidgetState::Clicked,
                ));
                return true;
            }
        }
        if let TheEvent::FileRequesterResult(id, _) = event {
            if id.name == "Palette Import" || id.name == "actionImportPalette" {
                self.receiver = None;
                self.source_label = None;
            }
        }
        if let TheEvent::StateChanged(id, TheWidgetState::Clicked) = event {
            if id.name == "Palette Dock Lospec" {
                Self::lospec_dialog(ui, ctx);
                return true;
            }
        }
        if let TheEvent::DialogValueOnClose(role, name, _, value) = event {
            if name == "Load Lospec Palette" {
                if *role == TheDialogButtonRole::Accept {
                    if let TheValue::Text(input) = value {
                        match lospec::slug(input) {
                            Ok(slug) => {
                                let (tx, rx) = std::sync::mpsc::channel();
                                self.receiver = Some(rx);
                                std::thread::spawn(move || {
                                    let _ = tx.send(lospec::fetch(&slug));
                                });
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Loading Lospec palette…".into(),
                                ));
                            }
                            Err(e) => ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), e)),
                        }
                    }
                }
                return true;
            }
        }
        let contents = match event {
            TheEvent::FileRequesterResult(id, paths)
                if id.name == "Palette Import" || id.name == "actionImportPalette" =>
            {
                paths
                    .first()
                    .map(|p| std::fs::read_to_string(p).map_err(|e| e.to_string()))
            }
            TheEvent::Custom(id, TheValue::Text(text)) if id.name == "Preview Palette Switch" => {
                Some(Ok(text.clone()))
            }
            _ => None,
        };
        if let Some(contents) = contents {
            match contents
                .and_then(|t| shared::palette::parse_colors(&t))
                .and_then(|colors| PaletteSwitch::prepare(project, colors, &ramps(project)))
            {
                Ok(mut plan) => {
                    if let Some(label) = self.source_label.take() {
                        plan.after.source_name = label;
                    }

                    let width = 700;
                    let height = 390;
                    let mut canvas = TheCanvas::new();
                    canvas.limiter_mut().set_max_size(Vec2::new(width, height));
                    let mut preview = TheRGBAView::new(TheId::named("Dialog Value"));
                    preview.set_mode(TheRGBAViewMode::Display);
                    preview.set_dont_show_grid(true);
                    preview.set_buffer(preview_sheet(project, &plan, ui.style.theme().as_ref()));
                    preview.set_zoom(1.);
                    canvas.set_widget(preview);
                    self.pending = Some(plan);
                    ui.show_dialog(
                        DIALOG,
                        canvas,
                        vec![TheDialogButtonRole::Accept, TheDialogButtonRole::Reject],
                        ctx,
                    );
                    if let Some(button) = ui.get_widget("TheDialogButtonRole::Accept") {
                        button.set_value(TheValue::Text("Apply Palette".into()));
                    }
                }
                Err(e) => ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), e)),
            }
            return true;
        }
        if let TheEvent::DialogValueOnClose(role, name, _, _) = event {
            if name == DIALOG {
                if let Some(plan) = self.pending.take() {
                    if *role == TheDialogButtonRole::Accept {
                        // A modal preview never mutates the live project. Reject stale plans.
                        if PaletteState::capture(project) != plan.before {
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                "Palette changed while previewing; import again.".into(),
                            ));
                            return true;
                        }
                        plan.after.apply(project);
                        UNDOMANAGER.write().unwrap().add_undo(
                            ProjectUndoAtom::PaletteSwitch(
                                Box::new(plan.before),
                                Box::new(plan.after),
                            ),
                            ctx,
                        );
                        crate::undo::project_helper::apply_palette(ui, ctx, server, project);
                        crate::undo::project_helper::refresh_palette_runtime(project);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Palette Changed"),
                            TheValue::Empty,
                        ));
                    }
                }
                return true;
            }
        }
        false
    }
}
fn preview_sheet(project: &Project, plan: &PaletteSwitch, theme: &dyn TheTheme) -> TheRGBABuffer {
    const W: usize = 700;
    const H: usize = 360;
    let bg = *theme.color(MaterialCanvas);
    let fg = *theme.color(MaterialText);
    let muted = *theme.color(MaterialMuted);
    let panel = *theme.color(MaterialPanel);
    let mut pixels = bg.repeat(W * H);
    let draw = TheDraw2D::new();
    let text = |pixels: &mut [u8], x: usize, y: usize, w: usize, label: &str, color: &[u8; 4]| {
        draw.text_rect(
            pixels,
            &(x, y, w, 22),
            W,
            label,
            TheFontSettings {
                size: 13.,
                ..Default::default()
            },
            color,
            &bg,
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        )
    };
    text(
        &mut pixels,
        16,
        8,
        660,
        &format!(
            "{} · {} colors · {} changed slots",
            plan.after.source_name,
            plan.after.source_colors.len(),
            plan.changed_slots
        ),
        &fg,
    );
    text(
        &mut pixels,
        16,
        31,
        660,
        "Slots and material settings stay assigned. Accept applies one undoable change.",
        &muted,
    );
    let bundled =
        shared::rulesets::bundled_tile_ids_for_project(&project.config).unwrap_or_default();
    let tiles = project
        .tiles
        .iter()
        .filter(|(id, _)| !bundled.contains(id))
        .map(|(_, t)| t)
        .filter(|t| t.role != rusterix::TileRole::Icon)
        .filter_map(|t| t.textures.first())
        .take(4)
        .cloned()
        .collect::<Vec<_>>();
    let icons = shared::rulesets::bundled_item_icon_states_for_project(&project.config)
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, state, _)| *state == "on")
        .filter_map(|(_, _, frames)| frames.into_iter().next())
        .take(4)
        .collect::<Vec<_>>();
    let mut assets = rusterix::Assets::default();
    if let Ok(avatars) = shared::rulesets::bundled_avatars_for_project(&project.config) {
        for (id, avatar) in avatars {
            assets.avatars.insert(id.into(), avatar);
        }
    }
    for (column, state) in [&plan.before, &plan.after].into_iter().enumerate() {
        let x0 = 16 + column * 342;
        let palette = &state.palette;
        text(
            &mut pixels,
            x0,
            64,
            330,
            if column == 0 {
                "Current"
            } else {
                "New palette"
            },
            &fg,
        );
        for (i, color) in palette.colors.iter().take(64).enumerate() {
            if let Some(color) = color {
                let x = x0 + (i % 16) * 20;
                let y = 91 + (i / 16) * 12;
                let rgba = color.to_u8_array();
                for yy in y..y + 10 {
                    for xx in x..x + 18 {
                        let at = (yy * W + xx) * 4;
                        pixels[at..at + 4].copy_from_slice(&rgba);
                    }
                }
            }
        }
        text(
            &mut pixels,
            x0,
            142,
            330,
            "Materials (cached preview)",
            &muted,
        );
        for (i, tile) in tiles.iter().enumerate() {
            let mut tile = tile.clone();
            if column == 1 {
                shared::palette::fit_rgba(&mut tile.data, palette);
            }
            stamp(
                &mut pixels,
                W,
                H,
                &tile.data,
                tile.width,
                tile.height,
                x0 + i * 62,
                168,
                54,
                panel,
            );
        }
        text(
            &mut pixels,
            x0,
            226,
            330,
            "Item artwork · Avatar · Game UI colors",
            &muted,
        );
        for (i, icon) in icons.iter().take(3).enumerate() {
            let mut icon = icon.clone();
            shared::palette::fit_rgba(&mut icon.data, palette);
            stamp(
                &mut pixels,
                W,
                H,
                &icon.data,
                icon.width,
                icon.height,
                x0 + i * 52,
                253,
                46,
                panel,
            );
        }
        assets.palette = palette.clone();
        if let Some(avatar) = assets
            .avatars
            .get("humanoid")
            .or_else(|| assets.avatars.values().next())
        {
            let mut entity = rusterix::Entity::default();
            for (key, index) in [
                ("light_skin_index", 22),
                ("dark_skin_index", 20),
                ("hair_index", 4),
                ("hands_index", 22),
                ("torso_index", 41),
                ("legs_index", 12),
            ] {
                entity.attributes.set(key, rusterix::Value::Int(index));
            }
            if let Some(frame) =
                rusterix::avatar_builder::AvatarRuntimeBuilder::build_preview_for_entity(
                    &entity,
                    avatar,
                    &assets,
                    None,
                    rusterix::AvatarDirection::Front,
                    0,
                    Default::default(),
                )
            {
                stamp(
                    &mut pixels,
                    W,
                    H,
                    &frame.rgba,
                    frame.size as usize,
                    frame.size as usize,
                    x0 + 160,
                    248,
                    64,
                    panel,
                );
            }
        }
        let colors = [4usize, 7, 8, 24, 41];
        for (i, index) in colors.into_iter().enumerate() {
            if let Some(c) = palette.colors.get(index).and_then(Option::as_ref) {
                let rgba = c.to_u8_array();
                for y in 255 + i * 11..264 + i * 11 {
                    for x in x0 + 244..x0 + 320 {
                        let at = (y * W + x) * 4;
                        pixels[at..at + 4].copy_from_slice(&rgba);
                    }
                }
            }
        }
    }
    let message = if plan.collapsed_ramps > 0 {
        format!(
            "{} shading ramps lose distinct stops in this palette.",
            plan.collapsed_ramps
        )
    } else {
        "Shading ramps retain ordered tones. All imported swatches remain available.".into()
    };
    text(&mut pixels, 16, 330, 670, &message, &muted);
    TheRGBABuffer::from(pixels, W as u32, H as u32)
}
fn stamp(
    dst: &mut [u8],
    w: usize,
    h: usize,
    src: &[u8],
    sw: usize,
    sh: usize,
    x0: usize,
    y0: usize,
    size: usize,
    bg: [u8; 4],
) {
    if sw == 0 || sh == 0 {
        return;
    }
    for y in 0..size {
        for x in 0..size {
            if x0 + x >= w || y0 + y >= h {
                continue;
            }
            let p = &src[((y * sh / size) * sw + x * sw / size) * 4..][..4];
            let alpha = p[3] as f32 / 255.;
            let at = ((y0 + y) * w + x0 + x) * 4;
            for c in 0..3 {
                dst[at + c] = (p[c] as f32 * alpha + bg[c] as f32 * (1. - alpha)) as u8;
            }
            dst[at + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plan(project: &Project) -> PaletteSwitch {
        PaletteSwitch::prepare(
            project,
            shared::palette::parse_colors("000000\n555555\naaaaaa\nffffff").unwrap(),
            &[vec![2, 5, 7, 8]],
        )
        .unwrap()
    }
    #[test]
    #[ignore = "runtime palette uploads require a native GPU"]
    fn cancel_keeps_live_project_and_accept_can_undo_the_whole_switch() {
        let mut project = Project::default();
        project.sync_ruleset_items().unwrap();
        let before = PaletteState::capture(&project);
        let switch = plan(&project);
        let after = switch.after.clone();
        let mut controller = PaletteSwitchController {
            pending: Some(switch),
            ..Default::default()
        };
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(800, 600, 1.);
        let mut server = ServerContext::default();
        assert!(controller.handle_event(
            &TheEvent::DialogValueOnClose(
                TheDialogButtonRole::Reject,
                DIALOG.into(),
                Uuid::new_v4(),
                TheValue::Empty
            ),
            &mut project,
            &mut ui,
            &mut ctx,
            &mut server
        ));
        assert_eq!(PaletteState::capture(&project), before);
        controller.pending = Some(plan(&project));
        assert!(controller.handle_event(
            &TheEvent::DialogValueOnClose(
                TheDialogButtonRole::Accept,
                DIALOG.into(),
                Uuid::new_v4(),
                TheValue::Empty
            ),
            &mut project,
            &mut ui,
            &mut ctx,
            &mut server
        ));
        assert_eq!(PaletteState::capture(&project), after);
        let atom =
            ProjectUndoAtom::PaletteSwitch(Box::new(before.clone()), Box::new(after.clone()));
        atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(PaletteState::capture(&project), before);
        atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(PaletteState::capture(&project), after);
    }
    #[test]
    fn comparison_preview_uses_a_dialog_value_and_does_not_edit_project() {
        let mut project = Project::default();
        let before = PaletteState::capture(&project);
        let mut controller = PaletteSwitchController::default();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(800, 600, 1.);
        let mut server = ServerContext::default();
        assert!(controller.handle_event(
            &TheEvent::Custom(
                TheId::named("Preview Palette Switch"),
                TheValue::Text("000000\nffffff".into())
            ),
            &mut project,
            &mut ui,
            &mut ctx,
            &mut server
        ));
        assert!(
            ui.get_widget("Dialog Value").is_some(),
            "TheUI needs a Dialog Value widget to deliver the close result"
        );
        assert_eq!(PaletteState::capture(&project), before);
    }
    #[test]
    #[ignore = "exports palette preview for visual inspection"]
    fn export_palette_switch_preview() {
        let mut project = Project::default();
        project.sync_ruleset_items().unwrap();
        crate::procedural_material::starters::install(&mut project).unwrap();
        for (id, bundle) in &project.procedural_materials {
            let asset =
                denrim_materials::integration::EditableMaterialAsset::reopen(bundle).unwrap();
            let program = asset
                .program(&denrim_materials::integration::InstanceRecord::new(
                    "preview",
                ))
                .unwrap();
            let cpu = denrim_materials::cpu::Evaluator::new(&program);
            let mut image = rusterix::Texture::alloc(16, 16);
            let srgb = |v: f32| {
                let v = v.clamp(0., 1.);
                let v = if v <= 0.0031308 {
                    v * 12.92
                } else {
                    1.055 * v.powf(1. / 2.4) - 0.055
                };
                (v * 255.).round() as u8
            };
            for y in 0..16 {
                for x in 0..16 {
                    let uv = [(x as f32 + 0.5) / 16., (y as f32 + 0.5) / 16.];
                    let color = cpu
                        .sample(
                            &asset.selected_material,
                            denrim_materials::ir::Channel::BaseColor,
                            uv,
                        )
                        .unwrap();
                    let at = (y * 16 + x) * 4;
                    image.data[at..at + 4].copy_from_slice(&[
                        srgb(color[0]),
                        srgb(color[1]),
                        srgb(color[2]),
                        255,
                    ]);
                }
            }
            project.tiles.get_mut(id).unwrap().textures = vec![image];
        }
        let colors=shared::palette::parse_colors("1a1c2c\n5d275d\nb13e53\nef7d57\nffcd75\na7f070\n38b764\n257179\n29366f\n3b5dc9\n41a6f6\n73eff7\nf4f4f4\n94b0c2\n566c86\n333c57").unwrap();
        let mut plan = PaletteSwitch::prepare(&project, colors, &ramps(&project)).unwrap();
        plan.after.source_name = "Sweetie 16".into();
        let image = preview_sheet(&project, &plan, &TheDarkTheme::new());
        let file = std::fs::File::create("/tmp/eldiron-palette-switch.png").unwrap();
        let mut encoder = png::Encoder::new(file, 700, 360);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(image.pixels())
            .unwrap();
    }
}
