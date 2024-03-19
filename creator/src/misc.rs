use crate::editor::CODEEDITOR;
use crate::prelude::*;
use std::time::{Duration, Instant};
use theframework::prelude::*;

pub fn create_code_menu(ctx: &mut TheContext) -> TheContextMenu {
    let mut code_menu = TheContextMenu::named(str!("Code"));

    code_menu.add(
        CODEEDITOR
            .lock()
            .unwrap()
            .create_keywords_context_menu_item(),
    );
    code_menu.add(
        CODEEDITOR
            .lock()
            .unwrap()
            .create_operators_context_menu_item(),
    );
    code_menu.add(CODEEDITOR.lock().unwrap().create_values_context_menu_item());

    let mut function_menu = TheContextMenu::new();

    let mut function_item =
        TheContextMenuItem::new(str!("Functions"), TheId::named("Code Functions"));

    let mut server_item = CODEEDITOR
        .lock()
        .unwrap()
        .create_functions_context_menu_item();
    server_item.name = "Server".to_string();
    function_menu.add(server_item);

    set_client_externals();

    let mut client_item = CODEEDITOR
        .lock()
        .unwrap()
        .create_functions_context_menu_item();
    client_item.name = "Client".to_string();
    function_menu.add(client_item);

    function_item.set_sub_menu(function_menu);
    code_menu.add(function_item);

    set_server_externals();

    code_menu.add(
        CODEEDITOR
            .lock()
            .unwrap()
            .create_modules_context_menu_item(),
    );

    CODEEDITOR.lock().unwrap().init_menu_selection(ctx);

    code_menu
}

pub struct UpdateTracker {
    //update_counter: u32,
    //last_fps_check: Instant,
    last_redraw_update: Instant,
    last_tick_update: Instant,
}

impl Default for UpdateTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateTracker {
    pub fn new() -> Self {
        UpdateTracker {
            //update_counter: 0,
            //last_fps_check: Instant::now(),
            last_redraw_update: Instant::now(),
            last_tick_update: Instant::now(),
        }
    }

    pub fn update(&mut self, redraw_ms: u64, tick_ms: u64) -> (bool, bool) {
        let mut redraw_update = false;
        let mut tick_update = false;

        // self.update_counter += 1;

        // if self.last_fps_check.elapsed() >= Duration::from_secs(1) {
        //     self.calculate_and_reset_fps();
        // }

        if self.last_redraw_update.elapsed() >= Duration::from_millis(redraw_ms) {
            self.last_redraw_update = Instant::now();
            redraw_update = true;
        }

        if self.last_tick_update.elapsed() >= Duration::from_millis(tick_ms) {
            self.last_tick_update = Instant::now();
            tick_update = true;
        }

        (redraw_update, tick_update)
    }

    // fn calculate_and_reset_fps(&mut self) {
    //     //let fps = self.update_counter;
    //     self.update_counter = 0;
    //     self.last_fps_check = Instant::now();
    //     //println!("FPS: {}", fps);
    // }
}
