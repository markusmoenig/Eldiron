#![cfg(feature = "ui")]

use theframework::prelude::*;

#[test]
fn dialog_dropdown_renders_and_selects_at_screen_coordinates() {
    for scale in [1.0, 2.0] {
        for (x, y) in [(180, 180), (560, 520)] {
            let mut ctx = TheContext::new(900, 600, scale);
            ctx.ui_render_scale = scale;
            let mut ui = TheUI::new();
            ui.init(&mut ctx);
            ui.canvas.root = true;
            let mut pixels = vec![0; 900 * 600 * 4];
            ui.draw(&mut pixels, &mut ctx);

            let mut dropdown = TheDropdownMenu::new(TheId::named("Dialog Dropdown"));
            dropdown.limiter_mut().set_max_width(220);
            for option in ["First", "Second", "Third", "Fourth", "Fifth"] {
                dropdown.add_option(option.into());
            }
            let mut form = TheTextLayout::new(TheId::named("Dialog Form"));
            form.add_pair("Choice".into(), Box::new(dropdown));
            let mut dialog = TheCanvas::new();
            dialog.limiter_mut().set_max_size(Vec2::new(320, 60));
            dialog.set_layout(form);
            ui.show_dialog("Dropdown regression", dialog, vec![], &mut ctx);
            ui.dialog
                .as_mut()
                .unwrap()
                .set_dim(TheDim::rect(x, y, 320, 60), &mut ctx);
            ui.draw(&mut pixels, &mut ctx);

            let control = *ui.get_widget("Dialog Dropdown").unwrap().dim();
            assert!(ui.touch_down((control.x + 10) as f32, (control.y + 10) as f32, &mut ctx));
            assert!(ctx.ui.overlay.is_some());
            ui.draw(&mut pixels, &mut ctx);

            let popup = ui
                .dialog
                .as_mut()
                .unwrap()
                .get_widget(Some(&"Dialog Dropdown".into()), None)
                .unwrap()
                .draw_overlay(&mut ui.style, &mut ctx);
            let dim = *popup.dim();
            assert!(dim.width >= control.width);
            assert_eq!(dim.x, control.x);
            if y == 180 {
                assert_eq!(dim.y, control.y + 20);
                assert!(dim.y + dim.height > y + 60);
            }
            assert!(dim.x >= 0 && dim.x + dim.width <= 900);
            assert!(dim.y >= 0 && dim.y + dim.height <= 600);
            // Compare the complete menu, including the part outside the dialog.
            for py in 0..dim.height {
                for px in 0..dim.width {
                    assert_eq!(
                        ui.canvas.buffer.get_pixel(dim.x + px, dim.y + py),
                        popup.get_pixel(px, py),
                        "popup pixel {px},{py}, dialog {x},{y}, scale {scale}",
                    );
                }
            }

            let target = Vec2::new(dim.x + 20, dim.y + 3 * 21 + 10);
            ui.touch_dragged(target.x as f32, target.y as f32, &mut ctx);
            ui.touch_up(target.x as f32, target.y as f32, &mut ctx);
            assert_eq!(
                ui.get_widget("Dialog Dropdown").unwrap().value(),
                TheValue::Int(3)
            );
            assert!(ctx.ui.overlay.is_none());
        }
    }
}
