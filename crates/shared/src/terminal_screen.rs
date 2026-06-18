use crate::project::Project;
use crate::region::Region;
use crate::screen::Screen;
use crate::text_game as sg;
use rusterix::{Entity, Item, Map, Value};
use toml::Table;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalRect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalWidget {
    pub name: String,
    pub role: String,
    pub rect: TerminalRect,
    pub data: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalScreenLayout {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub widgets: Vec<TerminalWidget>,
}

#[derive(Clone, Debug, Default)]
pub struct TerminalScreenFrame {
    pub header: String,
    pub hint: String,
    pub message: Option<String>,
}

pub fn project_terminal_screen_layout(project: &Project) -> Option<TerminalScreenLayout> {
    let start_screen = config_string(&project.config, "game", "start_screen", "");
    let screen = if start_screen.trim().is_empty() {
        project.screens.values().next()
    } else {
        project
            .screens
            .values()
            .find(|screen| screen.map.name == start_screen || screen.name == start_screen)
    }?;
    terminal_screen_layout(project, screen)
}

pub fn terminal_screen_layout(project: &Project, screen: &Screen) -> Option<TerminalScreenLayout> {
    let width = config_usize(&project.config, "viewport", "width", 80).max(1);
    let height = config_usize(&project.config, "viewport", "height", 24).max(1);
    let origin_x = -(width as f32) / 2.0;
    let origin_y = -(height as f32) / 2.0;

    let mut widgets = Vec::new();
    for sector in &screen.map.sectors {
        let data = sector.properties.get_str("data").unwrap_or("");
        let role = widget_role(data).unwrap_or_else(|| sector.name.to_ascii_lowercase());
        let bounds = sector.bounding_box(&screen.map);
        let x = ((bounds.min.x - origin_x).round() as isize).max(0) as usize;
        let y = ((bounds.min.y - origin_y).round() as isize).max(0) as usize;
        let mut widget_width = bounds.size().x.round().max(1.0) as usize;
        let mut widget_height = bounds.size().y.round().max(1.0) as usize;
        if x >= width || y >= height {
            continue;
        }
        widget_width = widget_width.min(width - x);
        widget_height = widget_height.min(height - y);
        widgets.push(TerminalWidget {
            name: sector.name.clone(),
            role,
            data: data.to_string(),
            rect: TerminalRect {
                x,
                y,
                width: widget_width,
                height: widget_height,
            },
        });
    }

    if widgets.is_empty() {
        return None;
    }

    Some(TerminalScreenLayout {
        name: screen.map.name.clone(),
        width,
        height,
        widgets,
    })
}

pub fn render_roguelike_screen(
    project: &Project,
    region: &Region,
    frame: &TerminalScreenFrame,
) -> String {
    let Some(layout) = project_terminal_screen_layout(project) else {
        return render_roguelike_view(region, frame);
    };

    let mut canvas = vec![vec![' '; layout.width]; layout.height];
    for widget in &layout.widgets {
        match widget.role.as_str() {
            "game" => draw_block(
                &mut canvas,
                &widget.rect,
                &terminal_widget_lines(widget, region, frame),
            ),
            "messages" => draw_message_widget(
                &mut canvas,
                widget,
                &terminal_widget_lines(widget, region, frame),
            ),
            "text" | "stat" | "avatar" => draw_text_block(
                &mut canvas,
                &widget.rect,
                &terminal_widget_lines(widget, region, frame),
            ),
            "button" => {
                draw_button_widget(&mut canvas, widget, region);
            }
            "deco" => {
                draw_deco_widget(&mut canvas, widget);
            }
            _ => {}
        }
    }

    trim_trailing_blank_lines(canvas)
        .into_iter()
        .map(|row| row.into_iter().collect::<String>().trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_roguelike_view(region: &Region, frame: &TerminalScreenFrame) -> String {
    let mut lines = Vec::new();
    if !frame.header.trim().is_empty() {
        lines.push(frame.header.clone());
    }
    if !frame.hint.trim().is_empty() {
        lines.push(frame.hint.clone());
    }
    if let Some(message) = &frame.message
        && !message.trim().is_empty()
    {
        lines.push(String::new());
        lines.extend(message.lines().map(str::to_string));
    }
    if !lines.is_empty() {
        lines.push(String::new());
    }
    match render_roguelike_map(&region.map) {
        Some(map_lines) => lines.extend(map_lines),
        None => lines.push(format!(
            "No source terrain metadata found for region '{}'.",
            region.map.name
        )),
    }
    lines.join("\n")
}

pub fn terminal_widget_lines(
    widget: &TerminalWidget,
    region: &Region,
    frame: &TerminalScreenFrame,
) -> Vec<String> {
    match widget.role.as_str() {
        "game" => render_roguelike_map(&region.map)
            .unwrap_or_else(|| vec![format!("No source terrain for '{}'.", region.name)]),
        "messages" => {
            let mut lines = Vec::new();
            if !frame.header.trim().is_empty() {
                lines.push(frame.header.clone());
            }
            if !frame.hint.trim().is_empty() {
                lines.push(frame.hint.clone());
            }
            if let Some(message) = &frame.message
                && !message.trim().is_empty()
            {
                lines.extend(message.lines().map(str::to_string));
            }
            lines
        }
        "text" => render_text_widget(widget, region),
        "stat" => render_stat_widget(widget, region),
        "avatar" => render_avatar_widget(widget, region),
        "button" => vec![widget_button_label(widget, region)],
        _ => Vec::new(),
    }
}

pub fn render_roguelike_map(map: &Map) -> Option<Vec<String>> {
    let mut terrain = source_terrain(map)?;
    for item in &map.items {
        let (x, y) = world_to_cell(item.position.x, item.position.z);
        put_glyph(&mut terrain, x, y, roguelike_item_glyph(item));
    }

    for entity in &map.entities {
        let (x, y) = world_to_cell(entity.position.x, entity.position.z);
        let glyph = if entity.is_player() {
            '@'
        } else {
            roguelike_entity_glyph(entity)
        };
        put_glyph(&mut terrain, x, y, glyph);
    }

    Some(
        terrain
            .into_iter()
            .map(|row| row.into_iter().collect())
            .collect(),
    )
}

pub fn source_terrain(map: &Map) -> Option<Vec<Vec<char>>> {
    map.sectors
        .iter()
        .find_map(|sector| sector.properties.get_str("eldiron_source_terrain"))
        .map(|terrain| {
            terrain
                .lines()
                .map(|line| line.chars().map(roguelike_base_glyph).collect())
                .collect()
        })
}

pub fn world_to_cell(x: f32, z: f32) -> (i32, i32) {
    (x.floor() as i32, z.floor() as i32)
}

pub fn is_roguelike_blocked(terrain: &[Vec<char>], x: i32, y: i32) -> bool {
    if x < 0 || y < 0 {
        return true;
    }
    let Some(row) = terrain.get(y as usize) else {
        return true;
    };
    let Some(tile) = row.get(x as usize) else {
        return true;
    };
    matches!(*tile, '#' | ' ')
}

fn draw_block(canvas: &mut [Vec<char>], rect: &TerminalRect, lines: &[String]) {
    for (line_index, line) in lines.iter().take(rect.height).enumerate() {
        let y = rect.y + line_index;
        let Some(row) = canvas.get_mut(y) else {
            continue;
        };
        for (col_index, ch) in line.chars().take(rect.width).enumerate() {
            let x = rect.x + col_index;
            if let Some(cell) = row.get_mut(x) {
                *cell = ch;
            }
        }
    }
}

fn draw_text_block(canvas: &mut [Vec<char>], rect: &TerminalRect, lines: &[String]) {
    let wrapped = wrap_lines(lines, rect.width);
    draw_block(canvas, rect, &wrapped);
}

fn draw_message_widget(canvas: &mut [Vec<char>], widget: &TerminalWidget, lines: &[String]) {
    if widget.rect.height == 0 || widget.rect.width == 0 {
        return;
    }

    draw_horizontal_rule(canvas, &widget.rect, &widget.name);
    if widget.rect.height <= 1 {
        return;
    }

    let content_rect = TerminalRect {
        x: widget.rect.x,
        y: widget.rect.y + 1,
        width: widget.rect.width,
        height: widget.rect.height - 1,
    };
    draw_text_block(canvas, &content_rect, lines);
}

fn draw_button_widget(canvas: &mut [Vec<char>], widget: &TerminalWidget, region: &Region) {
    let label = widget_button_label(widget, region);
    draw_box(canvas, &widget.rect, &label);
}

fn draw_deco_widget(canvas: &mut [Vec<char>], widget: &TerminalWidget) {
    draw_box(canvas, &widget.rect, "");
}

fn draw_box(canvas: &mut [Vec<char>], rect: &TerminalRect, label: &str) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    let x1 = rect.x + rect.width - 1;
    let y1 = rect.y + rect.height - 1;

    for y in rect.y..=y1 {
        let Some(row) = canvas.get_mut(y) else {
            continue;
        };
        for x in rect.x..=x1.min(row.len().saturating_sub(1)) {
            let edge = y == rect.y || y == y1 || x == rect.x || x == x1;
            if edge {
                row[x] = if (x == rect.x || x == x1) && (y == rect.y || y == y1) {
                    '+'
                } else if y == rect.y || y == y1 {
                    '-'
                } else {
                    '|'
                };
            }
        }
    }

    if rect.width <= 2 || rect.height <= 2 || label.trim().is_empty() {
        return;
    }
    let y = rect.y + rect.height / 2;
    let Some(row) = canvas.get_mut(y) else {
        return;
    };
    let available = rect.width.saturating_sub(2);
    let clipped: String = label.chars().take(available).collect();
    let start = rect.x + 1 + available.saturating_sub(clipped.chars().count()) / 2;
    for (index, ch) in clipped.chars().enumerate() {
        if let Some(cell) = row.get_mut(start + index) {
            *cell = ch;
        }
    }
}

fn draw_horizontal_rule(canvas: &mut [Vec<char>], rect: &TerminalRect, label: &str) {
    let Some(row) = canvas.get_mut(rect.y) else {
        return;
    };
    for x in rect.x..rect.x.saturating_add(rect.width).min(row.len()) {
        row[x] = '-';
    }

    let label = label.trim();
    if label.is_empty() || rect.width < 4 {
        return;
    }
    let decorated = format!(" {} ", label);
    for (index, ch) in decorated.chars().take(rect.width).enumerate() {
        if let Some(cell) = row.get_mut(rect.x + index) {
            *cell = ch;
        }
    }
}

fn render_text_widget(widget: &TerminalWidget, region: &Region) -> Vec<String> {
    let Some(ui) = widget_ui_table(widget) else {
        return Vec::new();
    };
    let Some(text) = ui.get("text").and_then(toml::Value::as_str) else {
        return Vec::new();
    };
    resolve_player_placeholders(text, region)
        .lines()
        .map(str::to_string)
        .collect()
}

fn render_stat_widget(widget: &TerminalWidget, region: &Region) -> Vec<String> {
    let Some(ui) = widget_ui_table(widget) else {
        return Vec::new();
    };
    let stat = ui.get("stat").and_then(toml::Value::as_str).unwrap_or("HP");
    let max_stat = ui
        .get("max_stat")
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("MAX_{}", stat));
    let Some(player) = current_player(region) else {
        return vec![format!("{}: -", stat)];
    };
    let current = player_attr_number(player, stat).unwrap_or(0.0);
    let max = player_attr_number(player, &max_stat).unwrap_or(current.max(1.0));
    let ratio = if max <= 0.0 {
        0.0
    } else {
        (current / max).clamp(0.0, 1.0)
    };
    let bar_width = widget
        .rect
        .width
        .saturating_sub(stat.len() + 8)
        .clamp(4, 20);
    let fill = (bar_width as f32 * ratio).round() as usize;
    let bar = format!(
        "[{}{}]",
        "#".repeat(fill),
        "-".repeat(bar_width.saturating_sub(fill))
    );
    vec![format!(
        "{} {} {}/{}",
        stat,
        bar,
        number_text(current),
        number_text(max)
    )]
}

fn render_avatar_widget(_widget: &TerminalWidget, region: &Region) -> Vec<String> {
    let Some(player) = current_player(region) else {
        return vec!["Avatar".to_string()];
    };
    let class = player_attr_text(player, "CLASS")
        .or_else(|| player_attr_text(player, "class_name"))
        .unwrap_or_else(|| "Player".to_string());
    let race = player_attr_text(player, "RACE").unwrap_or_default();
    if race.is_empty() {
        vec!["Avatar".to_string(), class]
    } else {
        vec!["Avatar".to_string(), format!("{} {}", race, class)]
    }
}

fn widget_button_label(widget: &TerminalWidget, region: &Region) -> String {
    let Some(ui) = widget_ui_table(widget) else {
        return widget.name.clone();
    };
    if let Some(text) = ui.get("text").and_then(toml::Value::as_str) {
        return text.to_string();
    }
    if let Some(intent) = ui.get("intent").and_then(toml::Value::as_str)
        && !intent.trim().is_empty()
    {
        return intent.to_string();
    }
    if let Some(command) = ui.get("command").and_then(toml::Value::as_str)
        && !command.trim().is_empty()
    {
        return command
            .rsplit(['.', ':'])
            .next()
            .unwrap_or(command)
            .to_string();
    }
    if let Some(slot) = ui.get("equipped_slot").and_then(toml::Value::as_str) {
        return current_player(region)
            .and_then(|player| player.get_equipped_item(slot))
            .map(sg::display_name_for_item)
            .unwrap_or_else(|| slot_label(slot));
    }
    if let Some(index) = ui.get("inventory_index").and_then(toml::Value::as_integer) {
        return current_player(region)
            .and_then(|player| player.get_item_in_slot(index.max(0) as usize))
            .map(sg::display_name_for_item)
            .unwrap_or_else(|| format!("Inv {}", index));
    }
    if let Some(slot) = ui.get("command_slot").and_then(toml::Value::as_str) {
        return slot_label(slot);
    }
    widget.name.clone()
}

fn widget_ui_table(widget: &TerminalWidget) -> Option<Table> {
    widget
        .data
        .parse::<Table>()
        .ok()
        .and_then(|table| table.get("ui").and_then(toml::Value::as_table).cloned())
}

fn current_player(region: &Region) -> Option<&Entity> {
    region.map.entities.iter().find(|entity| entity.is_player())
}

fn resolve_player_placeholders(text: &str, region: &Region) -> String {
    let Some(player) = current_player(region) else {
        return text.to_string();
    };

    let mut resolved = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{PLAYER.") {
        resolved.push_str(&rest[..start]);
        let after = &rest[start + "{PLAYER.".len()..];
        let Some(end) = after.find('}') else {
            resolved.push_str(&rest[start..]);
            return resolved;
        };
        let key = &after[..end];
        resolved.push_str(&player_placeholder(player, key));
        rest = &after[end + 1..];
    }
    resolved.push_str(rest);
    resolved
}

fn player_placeholder(player: &Entity, key: &str) -> String {
    match key.trim().to_ascii_uppercase().as_str() {
        "CLASS" => player_attr_text(player, "CLASS")
            .or_else(|| player_attr_text(player, "class"))
            .or_else(|| player_attr_text(player, "class_name"))
            .unwrap_or_default(),
        other => player_attr_text(player, other)
            .or_else(|| player_attr_number(player, other).map(number_text))
            .unwrap_or_default(),
    }
}

fn player_attr_text(player: &Entity, key: &str) -> Option<String> {
    player
        .attributes
        .get_str(key)
        .or_else(|| player.attributes.get_str(&key.to_ascii_lowercase()))
        .map(str::to_string)
}

fn player_attr_number(player: &Entity, key: &str) -> Option<f32> {
    player
        .attributes
        .get_float(key)
        .or_else(|| player.attributes.get_float(&key.to_ascii_uppercase()))
        .or_else(|| player.attributes.get_float(&key.to_ascii_lowercase()))
        .or_else(|| {
            player
                .attributes
                .get(key)
                .or_else(|| player.attributes.get(&key.to_ascii_uppercase()))
                .or_else(|| player.attributes.get(&key.to_ascii_lowercase()))
                .and_then(|value| match value {
                    Value::Str(value) => value.parse::<f32>().ok(),
                    _ => None,
                })
        })
}

fn number_text(value: f32) -> String {
    if (value.round() - value).abs() < f32::EPSILON {
        format!("{}", value.round() as i32)
    } else {
        format!("{:.1}", value)
    }
}

fn slot_label(slot: &str) -> String {
    slot.split(['_', '.', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn wrap_lines(lines: &[String], width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }

    let mut wrapped = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            wrapped.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in line.split_whitespace() {
            if current.is_empty() {
                current.push_str(word);
            } else if current.chars().count() + 1 + word.chars().count() <= width {
                current.push(' ');
                current.push_str(word);
            } else {
                wrapped.push(current);
                current = word.to_string();
            }

            while current.chars().count() > width {
                let rest: String = current.chars().skip(width).collect();
                current = current.chars().take(width).collect();
                wrapped.push(current);
                current = rest;
            }
        }
        if !current.is_empty() {
            wrapped.push(current);
        }
    }
    wrapped
}

fn trim_trailing_blank_lines(mut canvas: Vec<Vec<char>>) -> Vec<Vec<char>> {
    while canvas
        .last()
        .map(|row| row.iter().all(|ch| *ch == ' '))
        .unwrap_or(false)
    {
        canvas.pop();
    }
    canvas
}

fn widget_role(data: &str) -> Option<String> {
    data.parse::<Table>().ok().and_then(|table| {
        table
            .get("ui")
            .and_then(toml::Value::as_table)
            .and_then(|ui| ui.get("role"))
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|role| !role.is_empty())
            .map(|role| role.to_ascii_lowercase())
    })
}

fn config_table(src: &str) -> Option<Table> {
    src.parse::<Table>().ok()
}

fn config_string(src: &str, section: &str, key: &str, default: &str) -> String {
    config_table(src)
        .and_then(|table| {
            table
                .get(section)
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get(key))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| default.to_string())
}

fn config_usize(src: &str, section: &str, key: &str, default: usize) -> usize {
    config_table(src)
        .and_then(|table| {
            table
                .get(section)
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get(key))
                .and_then(toml::Value::as_integer)
        })
        .filter(|value| *value > 0)
        .map(|value| value as usize)
        .unwrap_or(default)
}

fn roguelike_base_glyph(glyph: char) -> char {
    if glyph == '@' || glyph.is_ascii_alphabetic() {
        '.'
    } else {
        glyph
    }
}

fn put_glyph(terrain: &mut [Vec<char>], x: i32, y: i32, glyph: char) {
    if x < 0 || y < 0 {
        return;
    }
    if let Some(row) = terrain.get_mut(y as usize)
        && let Some(cell) = row.get_mut(x as usize)
    {
        *cell = glyph;
    }
}

fn roguelike_item_glyph(item: &Item) -> char {
    let name = sg::display_name_for_item(item).to_ascii_lowercase();
    if name.contains("blessed") {
        'b'
    } else if name.contains("herb") {
        'h'
    } else {
        name.chars()
            .find(|ch| ch.is_ascii_alphanumeric())
            .unwrap_or('!')
            .to_ascii_lowercase()
    }
}

fn roguelike_entity_glyph(entity: &Entity) -> char {
    sg::display_name_for_entity(entity)
        .chars()
        .find(|ch| ch.is_ascii_alphanumeric())
        .unwrap_or('C')
        .to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use rusterix::{MapCamera, PixelSource, Value};

    #[test]
    fn renders_widgets_from_screen_roles() {
        let mut project = Project::default();
        project.config =
            "[game]\nstart_screen = \"play\"\n\n[viewport]\nwidth = 12\nheight = 6\n".to_string();

        let mut screen = Screen::new();
        screen.map.name = "play".to_string();
        screen.map.camera = MapCamera::TwoD;
        add_widget(&mut screen.map, "Game", "game", -6.0, -3.0, 12.0, 4.0);
        add_widget(
            &mut screen.map,
            "Messages",
            "messages",
            -6.0,
            1.0,
            12.0,
            2.0,
        );
        project.screens.insert(screen.id, screen);

        let mut region = Region::default();
        region.name = "Cellar".to_string();
        region.map.name = "cellar".to_string();
        add_terrain_sector(&mut region.map, "##\n#@");

        let frame = TerminalScreenFrame {
            header: "Cellar [roguelike]".to_string(),
            hint: "hints".to_string(),
            message: None,
        };
        let rendered = render_roguelike_screen(&project, &region, &frame);
        assert!(rendered.contains("##"));
        assert!(rendered.contains("Cellar"));
    }

    fn add_widget(map: &mut Map, name: &str, role: &str, x: f32, y: f32, w: f32, h: f32) {
        let v0 = map.add_vertex_at(x, y);
        let v1 = map.add_vertex_at(x + w, y);
        let v2 = map.add_vertex_at(x + w, y + h);
        let v3 = map.add_vertex_at(x, y + h);
        map.create_linedef_manual(v0, v1);
        map.create_linedef_manual(v1, v2);
        map.create_linedef_manual(v2, v3);
        map.create_linedef_manual(v3, v0);
        let sector_id = map.close_polygon_manual().unwrap();
        let sector = map.find_sector_mut(sector_id).unwrap();
        sector.name = name.to_string();
        sector
            .properties
            .set("data", Value::Str(format!("[ui]\nrole = \"{}\"\n", role)));
    }

    fn add_terrain_sector(map: &mut Map, terrain: &str) {
        let v0 = map.add_vertex_at(0.0, 0.0);
        let v1 = map.add_vertex_at(2.0, 0.0);
        let v2 = map.add_vertex_at(2.0, 2.0);
        let v3 = map.add_vertex_at(0.0, 2.0);
        map.create_linedef_manual(v0, v1);
        map.create_linedef_manual(v1, v2);
        map.create_linedef_manual(v2, v3);
        map.create_linedef_manual(v3, v0);
        let sector_id = map.close_polygon_manual().unwrap();
        let sector = map.find_sector_mut(sector_id).unwrap();
        sector
            .properties
            .set("eldiron_source_terrain", Value::Str(terrain.to_string()));
        sector
            .properties
            .set("source", Value::Source(PixelSource::Off));
    }
}
