//! Offscreen Creator theme review using native controls.
use theframework::prelude::*;
fn panel(theme: Box<dyn TheTheme>, title: &str) -> TheRGBABuffer {
    let mut ctx = TheContext::new(560, 380, 1.);
    let mut out = TheRGBABuffer::new(TheDim::sized(560, 380));
    out.fill(*theme.color(EditorCanvasBackground));
    let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(theme));
    let mut heading = TheText::new(TheId::empty());
    heading.set_text(title.into());
    heading.set_dim(TheDim::rect(12, 4, 530, 24), &mut ctx);
    heading.draw(&mut out, &mut style, &mut ctx);
    for (i, text) in ["Regions", "New Region", "Characters", "Items", "Tilesets"]
        .iter()
        .enumerate()
    {
        let mut row = TheSnapperbar::new(TheId::empty());
        row.set_text((*text).into());
        row.set_open(i < 2);
        row.set_selected(i == 1);
        row.set_dim(
            TheDim::rect(
                if i == 1 { 26 } else { 10 },
                40 + i as i32 * 26,
                if i == 1 { 204 } else { 220 },
                24,
            ),
            &mut ctx,
        );
        row.draw(&mut out, &mut style, &mut ctx);
    }
    let mut edit = TheTextLineEdit::new(TheId::empty());
    edit.set_value(TheValue::Text("New Region".into()));
    edit.set_dim(TheDim::rect(250, 40, 295, 24), &mut ctx);
    edit.draw(&mut out, &mut style, &mut ctx);
    let mut menu = TheDropdownMenu::new(TheId::empty());
    for text in ["All materials", "Noise", "Shapes"] {
        menu.add_option(text.into());
    }
    menu.set_dim(TheDim::rect(250, 80, 295, 24), &mut ctx);
    menu.draw(&mut out, &mut style, &mut ctx);
    let mut action = TheTraybarButton::new(TheId::empty());
    action.set_text("Make Proc".into());
    action.set_dim(TheDim::rect(250, 120, 130, 24), &mut ctx);
    action.draw(&mut out, &mut style, &mut ctx);
    for (i, text) in ["Sunstone Outcrop", "Settings", "World / Behavior Nodes"]
        .iter()
        .enumerate()
    {
        let mut row = TheSnapperbar::new(TheId::empty());
        row.set_text((*text).into());
        row.set_background_palette(TheThemePalettes::ActionGroups, i + 1);
        row.set_dim(TheDim::rect(10, 212 + i as i32 * 28, 220, 24), &mut ctx);
        row.draw(&mut out, &mut style, &mut ctx);
    }
    let stride = out.stride();
    for x in (250..550).step_by(28) {
        ctx.draw.rect(
            out.draw_target(),
            &(x, 212, 1, 156),
            stride,
            style.theme().color(EditorGrid),
        );
    }
    for y in (212..375).step_by(28) {
        ctx.draw.rect(
            out.draw_target(),
            &(250, y, 296, 1),
            stride,
            style.theme().color(EditorGrid),
        );
    }
    out
}
fn main() {
    let mut out = TheRGBABuffer::new(TheDim::sized(1120, 380));
    out.copy_into(0, 0, &panel(Box::new(TheBrownTheme::new()), "Brown"));
    out.copy_into(560, 0, &panel(Box::new(TheLightTheme::new()), "Light"));
    std::fs::write("/tmp/eldiron-theme-review.png", out.to_png().unwrap()).unwrap();
}
