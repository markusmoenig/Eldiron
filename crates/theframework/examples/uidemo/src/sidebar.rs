use crate::prelude::*;
use rust_pathtracer::prelude::*;
use std::sync::mpsc;

pub struct Sidebar {
    stack_layout_id: TheId,

    pub renderer_command: Option<mpsc::Sender<RendererMessage>>,
}

#[allow(clippy::new_without_default)]
impl Sidebar {
    pub fn new() -> Self {
        Self {
            stack_layout_id: TheId::empty(),

            renderer_command: None,
        }
    }

    pub fn init_ui(&mut self, ui: &mut TheUI, _ctx: &mut TheContext, project: &mut Project) {
        let width = 400;

        // Section Buttons

        // let mut sectionbar_canvas = TheCanvas::new();

        // let mut section_bar_canvas = TheCanvas::new();
        // section_bar_canvas.set_widget(TheSectionbar::new(TheId::named("Sectionbar")));
        // sectionbar_canvas.set_top(section_bar_canvas);

        // let mut sphere_sectionbar_button = TheSectionbarButton::new(TheId::named("Sphere Section"));
        // sphere_sectionbar_button.set_text("Sphere".to_string());
        // sphere_sectionbar_button.set_state(TheWidgetState::Selected);

        // let mut cube_sectionbar_button = TheSectionbarButton::new(TheId::named("Cube Section"));
        // cube_sectionbar_button.set_text("Cube".to_string());

        // let mut pyramid_sectionbar_button =
        //     TheSectionbarButton::new(TheId::named("Pyramid Section"));
        // pyramid_sectionbar_button.set_text("Pyramid".to_string());

        /*
        let mut vlayout = TheVLayout::new(TheId::named("Section Buttons"));
        vlayout.add_widget(Box::new(sphere_sectionbar_button));
        vlayout.add_widget(Box::new(cube_sectionbar_button));
        vlayout.add_widget(Box::new(pyramid_sectionbar_button));
        vlayout.set_margin(Vec4::new(5, 10, 5, 10));
        vlayout.set_padding(4);
        vlayout.set_background_color(Some(SectionbarBackground));
        vlayout.limiter_mut().set_max_width(90);
        sectionbar_canvas.set_layout(vlayout);
        */

        // Header

        let mut header: TheCanvas = TheCanvas::new();
        let mut switchbar = TheSwitchbar::new(TheId::empty());
        header.limiter_mut().set_max_width(400);
        switchbar.set_text("Material".to_string());
        header.set_widget(switchbar);

        // Stack Layout

        let mut stack_layout = TheStackLayout::new(TheId::named("Stack Layout"));
        stack_layout.limiter_mut().set_max_width(width);

        self.stack_layout_id = stack_layout.id().clone();

        // Material Canvas

        // let mut material_canvas = TheCanvas::default();

        /*
        let mut text_layout = TheTextLayout::new(TheId::named("Material Layout"));
        text_layout.limiter_mut().set_max_width(width);

        let mut color_picker = TheColorPicker::new(TheId::named("Color Picker"));
        color_picker.set_color(Vec3::new(
            project.material.rgb.x,
            project.material.rgb.y,
            project.material.rgb.z,
        ));
        text_layout.add_pair("".to_string(), Box::new(color_picker));

        let mut anisotropic = TheSlider::new(TheId::named("Anisotropic"));
        anisotropic.set_status_text("The anisotropic attribute of the material.");
        anisotropic.set_value(TheValue::Float(project.material.anisotropic));
        text_layout.add_pair("Anisotropic".to_string(), Box::new(anisotropic));

        let mut metallic = TheSlider::new(TheId::named("Metallic"));
        metallic.set_status_text("The metallic attribute of the material.");
        metallic.set_value(TheValue::Float(project.material.metallic));
        text_layout.add_pair("Metallic".to_string(), Box::new(metallic));

        let mut roughness = TheSlider::new(TheId::named("Roughness"));
        roughness.set_status_text("The roughness attribute of the material.");
        roughness.set_value(TheValue::Float(project.material.roughness));
        text_layout.add_pair("Roughness".to_string(), Box::new(roughness));

        let mut subsurface = TheSlider::new(TheId::named("Subsurface"));
        subsurface.set_status_text("The subsurface attribute of the material.");
        subsurface.set_value(TheValue::Float(project.material.subsurface));
        text_layout.add_pair("Subsurface".to_string(), Box::new(subsurface));

        let mut sheen = TheSlider::new(TheId::named("Sheen"));
        sheen.set_status_text("The sheen attribute of the material.");
        sheen.set_value(TheValue::Float(project.material.sheen));
        text_layout.add_pair("Sheen".to_string(), Box::new(sheen));

        let mut sheen_tint = TheSlider::new(TheId::named("Sheen Tint"));
        sheen_tint.set_status_text("The specular tint attribute of the material.");
        sheen_tint.set_value(TheValue::Float(project.material.sheen_tint));
        text_layout.add_pair("Sheen Tint".to_string(), Box::new(sheen_tint));

        let mut clearcoat = TheSlider::new(TheId::named("Clearcoat"));
        clearcoat.set_status_text("The clearcoat attribute of the material.");
        clearcoat.set_value(TheValue::Float(project.material.clearcoat));
        text_layout.add_pair("Clearcoat".to_string(), Box::new(clearcoat));

        let mut clearcoat_gloss = TheSlider::new(TheId::named("Clearcoat Gloss"));
        clearcoat_gloss.set_status_text("The clearcoat gloss attribute of the material.");
        clearcoat_gloss.set_value(TheValue::Float(project.material.clearcoat_gloss));
        text_layout.add_pair("Clearcoat Gloss".to_string(), Box::new(clearcoat_gloss));

        let mut transmission = TheSlider::new(TheId::named("Transmission"));
        transmission.set_status_text("The transmission attribute of the material.");
        transmission.set_value(TheValue::Float(project.material.spec_trans));
        text_layout.add_pair("Transmission".to_string(), Box::new(transmission));

        let mut ior = TheSlider::new(TheId::named("IOR"));
        ior.set_status_text("The index of refraction attribute of the material.");
        ior.set_range(TheValue::RangeF32(0.0..=2.0));
        ior.set_value(TheValue::Float(project.material.ior));
        text_layout.add_pair("IOR".to_string(), Box::new(ior));

        let mut emission = TheSlider::new(TheId::named("Emission"));
        emission.set_range(TheValue::RangeF32(0.0..=10.0));
        emission.set_status_text("The index of refraction attribute of the material.");
        emission.set_value(TheValue::Float(project.material.emission.x));
        text_layout.add_pair("Emission".to_string(), Box::new(emission));

        material_canvas.set_layout(text_layout);
        material_canvas.top_is_expanding = false;
        */
        // stack_layout.add_canvas(material_canvas);

        // Setup the TreeLayout

        let mut tree_layout = TheTreeLayout::new(TheId::named("Tree Layout"));

        let mut color_node: TheTreeNode = TheTreeNode::new(TheId::named("Color"));
        color_node.set_open(true);
        let mut color_picker = TheColorPicker::new(TheId::named("Color Picker"));
        color_picker.set_color(Vec3::new(
            project.material.rgb.x,
            project.material.rgb.y,
            project.material.rgb.z,
        ));
        color_node.add_widget(Box::new(color_picker));

        let mut parameters_node: TheTreeNode = TheTreeNode::new(TheId::named("Parameters"));
        parameters_node.set_open(true);

        let mut item = TheTreeItem::new(TheId::named("Anisotropic Item"));
        item.set_text("Anisotropic".into());
        item.set_status_text("The anisotropic attribute of the material.");

        let mut anisotropic = TheSlider::new(TheId::named("Anisotropic"));
        anisotropic.set_status_text("The anisotropic attribute of the material.");
        anisotropic.set_value(TheValue::Float(project.material.anisotropic));
        item.add_widget_column(250, Box::new(anisotropic));
        parameters_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Metallic Item"));
        item.set_text("Metallic".into());
        item.set_status_text("The metallic attribute of the material.");

        let mut metallic = TheTextLineEdit::new(TheId::named("Metallic"));
        metallic.set_range(TheValue::RangeF32(0.0..=1.0));
        // metallic.set_value(TheValue::Float(project.material.metallic));
        // metallic.set_value(TheValue::Text("0.0".into()));

        item.add_widget_column(250, Box::new(metallic));
        parameters_node.add_widget(Box::new(item));

        // for i in 0..100 {
        //     let mut item = TheTreeItem::new(TheId::named(&format!("Item #{}", i)));
        //     item.set_text(format!("Item {}", i));
        //     sub.add_widget(Box::new(item));
        // }

        let root = tree_layout.get_root();
        // root.add_widget(Box::new(item));

        root.add_child(color_node);
        root.add_child(parameters_node);

        tree_layout.limiter_mut().set_max_width(width);
        tree_layout.set_background_color(Some(TheThemeColors::DefaultWidgetDarkBackground));

        // Put it all into the canvas

        let mut canvas = TheCanvas::new();
        canvas.set_top(header);
        // stack_layout.set_index(0);
        canvas.top_is_expanding = false;
        canvas.set_layout(tree_layout);
        //canvas.set_right(sectionbar_canvas);

        ui.canvas.set_right(canvas);
    }

    #[allow(clippy::single_match)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::StateChanged(id, _state) => {
                //println!("app Widget State changed {:?}: {:?}", id, state);

                if id.name == "Open" {
                    #[cfg(not(target_arch = "wasm32"))]
                    ctx.ui.open_file_requester(
                        TheId::named("MyID"),
                        "Open".into(),
                        TheFileExtension::new("PNG".into(), vec!["png".to_string()]),
                    );
                    ctx.ui
                        .set_widget_state("Open".to_string(), TheWidgetState::None);
                    ctx.ui.clear_hover();
                } else if id.name == "Cube" {
                    ctx.ui
                        .set_widget_state("Sphere".to_string(), TheWidgetState::None);
                    ctx.ui
                        .send(TheEvent::SetStackIndex(self.stack_layout_id.clone(), 0));
                } else if id.name == "Sphere" {
                    ctx.ui
                        .set_widget_state("Cube".to_string(), TheWidgetState::None);
                    ctx.ui
                        .send(TheEvent::SetStackIndex(self.stack_layout_id.clone(), 1));
                }

                redraw = true;
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Color Picker" {
                    if let TheValue::ColorObject(v) = value {
                        let c = v.to_vec3();
                        project.material.rgb = rust_pathtracer::prelude::F3::new(c.x, c.y, c.z);
                        self.send_material(project.material.clone());
                    }
                } else if id.name == "Anisotropic" {
                    if let TheValue::Float(anisotropic) = value {
                        project.material.anisotropic = *anisotropic;
                        self.send_material(project.material.clone());
                    }
                } else if id.name == "Metallic" {
                    if let Some(value) = value.to_f32() {
                        project.material.metallic = value;
                        self.send_material(project.material.clone());
                    }
                } else if id.name == "Roughness" {
                    if let TheValue::Float(roughness) = value {
                        project.material.roughness = *roughness;
                        self.send_material(project.material.clone());
                    }
                } else if id.name == "Subsurface" {
                    if let TheValue::Float(subsurface) = value {
                        project.material.subsurface = *subsurface;
                        self.send_material(project.material.clone());
                    }
                } else if id.name == "Sheen" {
                    if let TheValue::Float(sheen) = value {
                        project.material.sheen = *sheen;
                        self.send_material(project.material.clone());
                    }
                } else if id.name == "Sheen Tint" {
                    if let TheValue::Float(sheen_tint) = value {
                        project.material.sheen_tint = *sheen_tint;
                        self.send_material(project.material.clone());
                    }
                } else if id.name == "Clearcoat" {
                    if let TheValue::Float(clearcoat) = value {
                        project.material.clearcoat = *clearcoat;
                        self.send_material(project.material.clone());
                    }
                } else if id.name == "Clearcoat Gloss" {
                    if let TheValue::Float(clearcoat_gloss) = value {
                        project.material.clearcoat_gloss = *clearcoat_gloss;
                        self.send_material(project.material.clone());
                    }
                }
                if id.name == "Transmission" {
                    if let TheValue::Float(transmission) = value {
                        project.material.spec_trans = *transmission;
                        self.send_material(project.material.clone());
                    }
                }
                if id.name == "IOR" {
                    if let TheValue::Float(ior) = value {
                        project.material.ior = *ior;
                        self.send_material(project.material.clone());
                    }
                }
                if id.name == "Emission" {
                    if let TheValue::Float(emission) = value {
                        project.material.emission = F3::new(
                            *emission * project.material.rgb.x,
                            *emission * project.material.rgb.y,
                            *emission * project.material.rgb.z,
                        );
                        self.send_material(project.material.clone());
                    }
                }
            }
            TheEvent::FileRequesterResult(id, paths) => {
                println!("FileRequester Result {:?} {:?}", id, paths);
            }
            _ => {}
        }
        redraw
    }

    // Sends the given material to the renderer.
    fn send_material(&mut self, material: Material) {
        if let Some(renderer_command) = &self.renderer_command {
            renderer_command
                .send(RendererMessage::Material(material))
                .unwrap();
        }
    }
}
