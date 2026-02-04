use crate::prelude::*;
use rust_pathtracer::prelude::*;
use std::sync::mpsc;
#[cfg(not(target_arch = "wasm32"))]
use std::thread::{self, JoinHandle};

// Define message types
pub enum RendererMessage {
    Start,
    Pause,
    Stop,
    Quit,
    Material(Material),
}

pub struct Renderer {
    width: i32,
    height: i32,

    #[cfg(not(target_arch = "wasm32"))]
    renderer_thread: Option<JoinHandle<()>>,
    pub renderer_command: Option<mpsc::Sender<RendererMessage>>,
    renderer_update: Option<mpsc::Receiver<TheRGBABuffer>>,
}

#[allow(clippy::new_without_default)]
impl Renderer {
    pub fn new() -> Self {
        let width = 787;
        let height = 596;

        Self {
            #[cfg(not(target_arch = "wasm32"))]
            renderer_thread: None,

            renderer_command: None,
            renderer_update: None,

            width,
            height,
        }
    }

    pub fn init_ui(&mut self, ui: &mut TheUI, _ctx: &mut TheContext, project: &mut Project) {
        let mut canvas = TheCanvas::new();

        // Toolbar

        let mut width_name_text = TheText::new(TheId::empty());
        width_name_text.set_text("Width".to_string());

        let mut width_name_edit = TheTextLineEdit::new(TheId::named("Width Edit"));
        width_name_edit.set_value(TheValue::Text(self.width.to_string()));
        width_name_edit.limiter_mut().set_max_width(100);

        let mut height_name_text = TheText::new(TheId::empty());
        height_name_text.set_text("Height".to_string());

        let mut height_name_edit = TheTextLineEdit::new(TheId::named("Height Edit"));
        height_name_edit.set_value(TheValue::Text(self.height.to_string()));
        height_name_edit.limiter_mut().set_max_width(100);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(width_name_text));
        toolbar_hlayout.add_widget(Box::new(width_name_edit));
        let mut spacer = TheSpacer::new(TheId::empty());
        spacer.limiter_mut().set_max_width(5);
        toolbar_hlayout.add_widget(Box::new(spacer));
        toolbar_hlayout.add_widget(Box::new(height_name_text));
        toolbar_hlayout.add_widget(Box::new(height_name_edit));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_top(toolbar_canvas);

        // RGBA Layout
        let mut rgba_layout = TheRGBALayout::new(TheId::named("RGBA Layout"));
        let buffer = TheRGBABuffer::new(TheDim::new(0, 0, self.width, self.height));
        rgba_layout.set_buffer(buffer);

        canvas.set_layout(rgba_layout);

        //

        ui.canvas.set_center(canvas);

        // Start the renderer thread
        self.renderer(project);
    }

    #[allow(clippy::single_match)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::StateChanged(_id, _state) => {
                //println!("app Widget State changed {:?}: {:?}", id, state);

                //if id.name == "Open" {
                redraw = true;
            }
            TheEvent::FileRequesterResult(id, paths) => {
                println!("FileRequester Result {:?} {:?}", id, paths);
            }
            _ => {}
        }
        redraw
    }

    /// Check if the renderer thread has an update for us.
    pub fn check_renderer_update(&mut self, ui: &mut TheUI, _ctx: &mut TheContext) -> bool {
        if let Some(receiver) = &self.renderer_update {
            while let Ok(buffer) = receiver.try_recv() {
                if let Some(layout) = ui.get_rgba_layout("RGBA Layout") {
                    if let Some(rgba_view) = layout.rgba_view_mut().as_rgba_view() {
                        rgba_view.set_buffer(buffer);
                        return true;
                    }
                }
            }
        }
        false
    }

    #[allow(clippy::single_match)]
    /// Starts a renderer thread which communicates via channels.
    pub fn renderer(&mut self, project: &mut Project) -> bool {
        let (command_sender, command_receiver) = mpsc::channel();
        let (update_sender, update_receiver) = mpsc::channel();

        self.renderer_command = Some(command_sender);
        self.renderer_update = Some(update_receiver);

        let width = self.width;
        let height = self.height;

        let mut buffer = ColorBuffer::new(width as usize, height as usize);

        let material = project.material.clone();
        let mut scene = Box::new(AnalyticalScene::new());
        scene.set_material(material);
        let mut tracer = Tracer::new(scene);

        #[cfg(target_arch = "wasm32")]
        {
            use std::cell::RefCell;
            use wasm_bindgen::prelude::*;

            let handler = Rc::new(RefCell::new(None));
            let handler_clone = handler.clone();

            *handler_clone.borrow_mut() = Some(Closure::new(move || {
                handle_render(
                    &command_receiver,
                    &mut tracer,
                    &mut buffer,
                    width,
                    height,
                    &update_sender,
                );

                request_animation_frame(handler.borrow().as_ref().unwrap());
            }));

            request_animation_frame(handler_clone.borrow().as_ref().unwrap());

            fn request_animation_frame(f: &Closure<dyn FnMut()>) {
                web_sys::window()
                    .unwrap()
                    .request_animation_frame(f.as_ref().unchecked_ref())
                    .unwrap();
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.renderer_thread = Some(thread::spawn(move || loop {
                handle_render(
                    &command_receiver,
                    &mut tracer,
                    &mut buffer,
                    width,
                    height,
                    &update_sender,
                );

                thread::sleep(core::time::Duration::from_millis(10));
            }));
        }

        return false;

        fn handle_render(
            command_receiver: &mpsc::Receiver<RendererMessage>,
            tracer: &mut Tracer,
            buffer: &mut ColorBuffer,
            width: i32,
            height: i32,
            update_sender: &mpsc::Sender<TheRGBABuffer>,
        ) {
            match command_receiver.try_recv() {
                Ok(message) => match message {
                    RendererMessage::Start => {}
                    RendererMessage::Material(material) => {
                        if let Some(update) = tracer
                            .scene()
                            .as_any()
                            .downcast_mut::<AnalyticalScene>()
                            .map(|external_widget| external_widget as &mut dyn UpdateTrait)
                        {
                            update.set_material(material);
                            buffer.frames = 0;
                        }
                    }
                    _ => {}
                },
                Err(_) => (),
            }

            tracer.render(buffer);

            let mut rgba_buffer = TheRGBABuffer::new(TheDim::new(0, 0, width, height));

            buffer.convert_to_u8(rgba_buffer.pixels_mut());
            update_sender.send(rgba_buffer).unwrap();
        }
    }
}
