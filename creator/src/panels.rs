use crate::editor::{
    CODEEDITOR, CODEGRIDFX, CONFIGEDITOR, INFOVIEWER, NODEEDITOR, RENDEREDITOR, SHADEGRIDFX,
    SHAPEPICKER, TILEMAPEDITOR, TILEPICKER, WORLDEDITOR,
};
use crate::prelude::*;

pub enum PanelIndices {
    TilePicker,
    TileMapEditor,
    ColorPicker,
    CodeEditor,
    DataEditor,
    // MaterialPicker,
    ShapePicker,
    ConfigEditor,
    InfoViewer,
    NodeEditor,
    TerrainBrush,
    Trace,
    CodeGridFx,
    ShadeGridFx,
}

pub struct Panels {}

#[allow(clippy::new_without_default)]
impl Panels {
    pub fn new() -> Self {
        Self {}
    }

    pub fn init_ui(
        &mut self,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut shared_layout = TheSharedHLayout::new(TheId::named("Shared Panel Layout"));
        shared_layout.set_shared_ratio(0.27);
        shared_layout.set_mode(TheSharedHLayoutMode::Right);

        // Main Stack

        let mut main_canvas = TheCanvas::new();
        let mut main_stack = TheStackLayout::new(TheId::named("Main Stack"));

        main_stack.add_canvas(TILEPICKER.write().unwrap().build(false));
        main_stack.add_canvas(TILEMAPEDITOR.write().unwrap().build());

        // Color Picker
        let mut color_picker_canvas: TheCanvas = TheCanvas::default();
        let mut color_picker_layout = TheHLayout::new(TheId::empty());

        let mut palette_picker = ThePalettePicker::new(TheId::named("Panel Palette Picker"));
        palette_picker
            .limiter_mut()
            .set_max_size(Vec2::new(500, 200));
        palette_picker.set_palette(project.palette.clone());
        palette_picker.set_rows_columns(10, 24);

        color_picker_layout.add_widget(Box::new(palette_picker));

        color_picker_layout.set_background_color(Some(DefaultWidgetDarkBackground));
        color_picker_layout.set_margin(Vec4::new(20, 5, 20, 5));

        color_picker_canvas.set_layout(color_picker_layout);
        main_stack.add_canvas(color_picker_canvas);

        // --

        main_stack.add_canvas(CODEEDITOR.write().unwrap().build());
        main_stack.add_canvas(CODEEDITOR.write().unwrap().build_data());
        main_stack.add_canvas(SHAPEPICKER.write().unwrap().build(false));
        main_stack.add_canvas(CONFIGEDITOR.write().unwrap().build());
        main_stack.add_canvas(INFOVIEWER.write().unwrap().build());
        main_stack.add_canvas(NODEEDITOR.write().unwrap().build());
        main_stack.add_canvas(WORLDEDITOR.write().unwrap().build_brush_canvas());
        main_stack.add_canvas(RENDEREDITOR.write().unwrap().build_trace_canvas());
        main_stack.add_canvas(
            CODEGRIDFX
                .write()
                .unwrap()
                .build_canvas(ctx, "CodeModuleView"),
        );
        main_stack.add_canvas(
            SHADEGRIDFX
                .write()
                .unwrap()
                .build_canvas(ctx, "ShadeModuleView"),
        );
        main_stack.set_index(0);

        let tilemap_editor = TheRGBALayout::new(TheId::named("Tilemap Editor"));
        let mut tilemap_canvas = TheCanvas::new();
        tilemap_canvas.set_layout(tilemap_editor);
        main_stack.add_canvas(tilemap_canvas);

        main_canvas.set_layout(main_stack);

        // Details Stack

        let mut details_canvas = TheCanvas::new();
        let mut details_stack = TheStackLayout::new(TheId::named("Details Stack"));

        // Context Group

        let mut context_group: TheGroupButton =
            TheGroupButton::new(TheId::named("Details Stack Group"));
        context_group.add_text_status(
            "Context".to_string(),
            "Shows the visual context of the selected code.".to_string(),
        );
        context_group.add_text_status(
            "Object".to_string(),
            "Shows the object properties.".to_string(),
        );
        context_group.add_text_status("Output".to_string(), "Shows the text output for the current character. Only available when the server is running.".to_string());

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 2, 5, 2));

        // let mut text = TheText::new(TheId::named("Panel Object Text"));
        // text.set_text("Object".to_string());
        toolbar_hlayout.add_widget(Box::new(context_group));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        details_canvas.set_top(toolbar_canvas);

        // Context

        let mut codecontext_canvas = TheCanvas::new();
        let codecontext_layout = TheListLayout::new(TheId::named("CodeObject Context Layout"));
        codecontext_canvas.set_layout(codecontext_layout);

        details_stack.add_canvas(codecontext_canvas);

        // Object

        let mut codeobject_canvas = TheCanvas::new();
        let codeobject_layout = TheListLayout::new(TheId::named("CodeObject Layout"));
        codeobject_canvas.set_layout(codeobject_layout);

        details_stack.add_canvas(codeobject_canvas);

        // Out

        let mut out_canvas = TheCanvas::new();

        let codeobject_layout = TheListLayout::new(TheId::named("CodeObject Output Layout"));
        out_canvas.set_layout(codeobject_layout);

        details_stack.add_canvas(out_canvas);

        //

        details_canvas.set_layout(details_stack);

        //

        shared_layout.add_canvas(details_canvas);
        shared_layout.add_canvas(main_canvas);

        canvas.set_layout(shared_layout);

        canvas
    }

    /*
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        return false;
        let mut redraw = false;
        if TILEPICKER
            .write()
            .unwrap()
            .handle_event(event, ui, ctx, project, server_ctx)
        {
            redraw = true;
        }
        // if MATERIALPICKER
        //     .write()
        //     .unwrap()
        //     .handle_event(event, ui, ctx, project, server_ctx)
        // {
        //     redraw = true;
        // }
        // if EFFECTPICKER
        //     .write()
        //     .unwrap()
        //     .handle_event(event, ui, ctx, project, server_ctx)
        // {
        //     redraw = true;
        // }
        if SHAPEPICKER
            .write()
            .unwrap()
            .handle_event(event, ui, ctx, project, server_ctx)
        {
            redraw = true;
        }

        // Shader, not sure we need this "if" here
        if server_ctx.curr_map_tool_helper == MapToolHelper::ShaderEditor
            && CODEEDITOR.read().unwrap().active_panel == VisibleCodePanel::Shade
        {
            SHADEGRIDFX
                .write()
                .unwrap()
                .handle_event(event, ui, ctx, &project.palette);
        }

        // Nodes
        if server_ctx.curr_map_tool_helper == MapToolHelper::NodeEditor
            && NODEEDITOR
                .write()
                .unwrap()
                .handle_event(event, ui, ctx, project, server_ctx)
        {
            redraw = true;
        }

        match event {
            /*
            TheEvent::StateChanged(id, _) => {
                if id.name == "cgfxAddToShaderLibrary" {
                    open_text_dialog(
                        "Add Shader To Library",
                        "Shader Name",
                        "Shader",
                        id.uuid,
                        ui,
                        ctx,
                    );
                }
            }*/
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Details Stack Group" {
                    if let Some(stack) = ui.get_stack_layout("Details Stack") {
                        stack.set_index(*index);
                        redraw = true;
                        ctx.ui.relayout = true;
                    }
                }
            }
            TheEvent::Custom(id, _) => {
                if id.name == "ModuleChanged"
                    && CODEEDITOR.read().unwrap().active_panel == VisibleCodePanel::Shade
                {
                    {
                        let mut module = SHADEGRIDFX.write().unwrap();
                        crate::utils::draw_shader_into(&module, &mut SHADERBUFFER.write().unwrap());

                        module.set_shader_background(SHADERBUFFER.read().unwrap().clone(), ui, ctx);
                    }

                    /*
                    match CODEEDITOR.read().unwrap().shader_content {
                        ContentContext::Sector(sector_id) => {
                            println!("sector");

                            if let Some(map) = project.get_map_mut(server_ctx) {
                                for s in &mut map.sectors {
                                    if s.creator_id == sector_id {
                                        //s.module = SHADEGRIDFX.read().unwrap().clone();
                                        SHADEGRIDFX.write().unwrap().redraw(ui, ctx);

                                        ctx.ui.send(TheEvent::Custom(
                                            TheId::named("Render SceneManager Map"),
                                            TheValue::Empty,
                                        ));

                                        RUSTERIX.write().unwrap().set_dirty();
                                        break;
                                    }
                                }
                            }
                        }
                        ContentContext::Shader(id) => {
                            println!("shader");
                            if let Some(shader) = project.shaders.get_mut(&id) {
                                *shader = SHADEGRIDFX.read().unwrap().clone();
                                crate::utils::draw_shader_into(
                                    shader,
                                    &mut SHADERBUFFER.write().unwrap(),
                                );

                                shader.set_shader_background(
                                    SHADERBUFFER.read().unwrap().clone(),
                                    ui,
                                    ctx,
                                );
                            }
                        }
                        _ => {}
                    }*/
                } else if id.name == "Set Region Modeler" {
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 4));
                    if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                        layout.set_mode(TheSharedHLayoutMode::Right);
                        ctx.ui.relayout = true;
                        redraw = true;
                    }
                    // MODELFXEDITOR
                    //     .lock()
                    //     .unwrap()
                    //     .activated(server_ctx, project, ui, ctx);
                } else if id.name == "Set Region Brush" {
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 5));
                    if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                        layout.set_mode(TheSharedHLayoutMode::Right);
                        ctx.ui.relayout = true;
                        redraw = true;
                    }
                    // MODELFXEDITOR
                    //     .lock()
                    //     .unwrap()
                    //     .activated(server_ctx, project, ui, ctx);
                } else if id.name == "Set Region Render" {
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 7));
                    if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                        layout.set_mode(TheSharedHLayoutMode::Right);
                        ctx.ui.relayout = true;
                        redraw = true;
                    }
                } else if id.name == "Set Tilemap Panel" {
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 1));
                    if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                        layout.set_mode(TheSharedHLayoutMode::Right);
                        ctx.ui.relayout = true;
                        redraw = true;
                    }
                } else if id.name == "Set Tilepicker Panel" {
                    ctx.ui
                        .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 0));
                    if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                        layout.set_mode(TheSharedHLayoutMode::Right);
                        ctx.ui.relayout = true;
                        redraw = true;
                    }
                }
            }
            _ => {}
        }

        redraw
    }*/

    /// Sets the brush panel.
    pub fn set_brush_panel(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(stack) = ui.get_stack_layout("Main Stack") {
            stack.set_index(5);
        }
        if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
            layout.set_mode(TheSharedHLayoutMode::Right);
            ctx.ui.relayout = true;
        }
    }
}
