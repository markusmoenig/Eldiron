use rusterix::{Map, MapCamera, PixelSource, Sector, Value};
use serde::Deserialize;
use shared::prelude::{Character, IndexMap, Item, Project, Region, Screen};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vek::Vec3;

#[derive(Debug, Deserialize)]
struct ProjectToml {
    #[serde(default)]
    project: ProjectSection,
    #[serde(default)]
    source: SourceSection,
    #[serde(default)]
    game: GameSection,
    #[serde(default)]
    viewport: ViewportSection,
    #[serde(default)]
    terminal: TerminalSection,
    #[serde(default)]
    build: BuildSection,
}

#[derive(Debug, Default, Deserialize)]
struct ProjectSection {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct SourceSection {
    #[serde(default = "default_main_source")]
    main: String,
}

impl Default for SourceSection {
    fn default() -> Self {
        Self {
            main: default_main_source(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GameSection {
    #[serde(default)]
    start_region: String,
    #[serde(default)]
    start_screen: String,
    #[serde(default = "default_terminal_mode")]
    terminal_mode: String,
    #[serde(default = "default_simulation_mode")]
    simulation_mode: String,
    #[serde(default = "default_turn_timeout_ms")]
    turn_timeout_ms: u32,
    #[serde(default = "default_collision_mode")]
    collision_mode: String,
    #[serde(default)]
    auto_create_player: bool,
    #[serde(default = "default_player")]
    player: String,
}

impl Default for GameSection {
    fn default() -> Self {
        Self {
            start_region: String::new(),
            start_screen: String::new(),
            terminal_mode: default_terminal_mode(),
            simulation_mode: default_simulation_mode(),
            turn_timeout_ms: default_turn_timeout_ms(),
            collision_mode: default_collision_mode(),
            auto_create_player: true,
            player: default_player(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ViewportSection {
    #[serde(default = "default_viewport_width")]
    width: u32,
    #[serde(default = "default_viewport_height")]
    height: u32,
    #[serde(default = "default_viewport_grid_size")]
    grid_size: u32,
    #[serde(default = "default_viewport_unit")]
    unit: String,
    #[serde(default = "default_viewport_resize")]
    resize: String,
}

impl Default for ViewportSection {
    fn default() -> Self {
        Self {
            width: default_viewport_width(),
            height: default_viewport_height(),
            grid_size: default_viewport_grid_size(),
            unit: default_viewport_unit(),
            resize: default_viewport_resize(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TerminalSection {
    #[serde(default = "default_terminal_text_updates")]
    text_updates: bool,
}

impl Default for TerminalSection {
    fn default() -> Self {
        Self {
            text_updates: default_terminal_text_updates(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct BuildSection {
    #[serde(default = "default_output")]
    output: String,
}

impl Default for BuildSection {
    fn default() -> Self {
        Self {
            output: default_output(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceCharacter {
    id: String,
    name: String,
    glyph: Option<char>,
    data: String,
    script: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceItem {
    id: String,
    name: String,
    glyph: Option<char>,
    data: String,
    script: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceRegion {
    id: String,
    name: String,
    default: String,
    terrain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceScreen {
    id: String,
    name: String,
    widgets: Vec<SourceWidget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceWidget {
    name: String,
    role: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    data: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct SourceDocument {
    characters: Vec<SourceCharacter>,
    items: Vec<SourceItem>,
    regions: Vec<SourceRegion>,
    screens: Vec<SourceScreen>,
}

impl SourceDocument {
    fn extend(&mut self, other: SourceDocument) {
        self.characters.extend(other.characters);
        self.items.extend(other.items);
        self.regions.extend(other.regions);
        self.screens.extend(other.screens);
    }
}

#[derive(Debug, Clone)]
struct Spawn {
    kind: SpawnKind,
    x: usize,
    y: usize,
}

#[derive(Debug, Clone)]
enum SpawnKind {
    Player,
    Character(char),
    Item(char),
}

pub fn build_project(project_dir: &Path) -> Result<PathBuf, String> {
    let config_path = project_dir.join("eldiron.toml");
    let config_text = fs::read_to_string(&config_path)
        .map_err(|err| format!("failed to read {}: {err}", config_path.display()))?;
    let config: ProjectToml = toml::from_str(&config_text)
        .map_err(|err| format!("failed to parse {}: {err}", config_path.display()))?;

    let source_path = project_dir.join(&config.source.main);
    let source_text = fs::read_to_string(&source_path)
        .map_err(|err| format!("failed to read {}: {err}", source_path.display()))?;
    let mut source = parse_source(&source_text)
        .map_err(|err| format!("failed to parse {}: {err}", source_path.display()))?;
    source.extend(load_source_dir(project_dir, "characters")?);
    source.extend(load_source_dir(project_dir, "items")?);

    let project = compile_project(&config, source)?;
    let output_path = project_dir.join(&config.build.output);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&project)
        .map_err(|err| format!("failed to serialize project: {err}"))?;
    fs::write(&output_path, json)
        .map_err(|err| format!("failed to write {}: {err}", output_path.display()))?;
    Ok(output_path)
}

fn load_source_dir(project_dir: &Path, name: &str) -> Result<SourceDocument, String> {
    let dir = project_dir.join(name);
    if !dir.exists() {
        return Ok(SourceDocument::default());
    }
    if !dir.is_dir() {
        return Err(format!("{} exists but is not a directory", dir.display()));
    }

    let mut paths = fs::read_dir(&dir)
        .map_err(|err| format!("failed to read {}: {err}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read {}: {err}", dir.display()))?;
    paths.retain(|path| path.extension().is_some_and(|extension| extension == "els"));
    paths.sort();

    let mut document = SourceDocument::default();
    for path in paths {
        let source_text = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        let parsed = parse_source(&source_text)
            .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
        document.extend(parsed);
    }
    Ok(document)
}

fn compile_project(config: &ProjectToml, source: SourceDocument) -> Result<Project, String> {
    let mut project = Project::new();
    project.name = if config.project.name.trim().is_empty() {
        "Eldiron Source Project".to_string()
    } else {
        config.project.name.trim().to_string()
    };
    project.regions.clear();
    project.characters.clear();
    project.items.clear();
    project.screens.clear();
    project.config = project_config(&config.game, &config.viewport, &config.terminal);
    project.migrate_default_ruleset();
    project.authoring = "[startup]\nshow = \"room\"\n".to_string();
    project.sync_ruleset_items()?;

    let mut character_templates = IndexMap::default();
    let mut item_templates = IndexMap::default();
    register_ruleset_item_glyphs(&project.items, &mut item_templates);
    for source_character in source.characters {
        let mut character = Character::new();
        character.name = source_character.name;
        character.source = source_character.script;
        character.source_debug = character.source.clone();
        character.data = if source_character.data.trim().is_empty() {
            character_data(&source_character.id, &config.game.player)
        } else {
            source_character.data
        };
        character.authoring = character_authoring(source_character.glyph);
        character.module = module_shell(
            "CharacterTemplate",
            &character.name,
            source_character.id == config.game.player,
        );
        let template_id = character.id;
        character_templates.insert(source_character.id.clone(), template_id);
        if let Some(glyph) = source_character.glyph {
            character_templates.insert(glyph.to_string(), template_id);
        }
        project.characters.insert(template_id, character);
    }

    for source_item in source.items {
        let mut item = Item::new();
        item.name = source_item.name;
        item.source = source_item.script;
        item.source_debug = item.source.clone();
        item.data = if source_item.data.trim().is_empty() {
            item_data(&source_item.id, source_item.glyph)
        } else {
            source_item.data
        };
        item.authoring = item_authoring(source_item.glyph);
        item.module = module_shell("ItemTemplate", &item.name, false);
        let template_id = item.id;
        item_templates.insert(source_item.id.clone(), template_id);
        if let Some(glyph) = source_item.glyph {
            item_templates.insert(glyph.to_string(), template_id);
        }
        project.items.insert(template_id, item);
    }

    ensure_player_template(&mut project, &mut character_templates, &config.game.player);

    for source_region in source.regions {
        project.regions.push(compile_region(
            source_region,
            &character_templates,
            &item_templates,
            &project.items,
            &config.game.player,
        )?);
    }

    for source_screen in source.screens {
        project.add_screen(compile_screen(source_screen, &config.viewport)?);
    }

    if project.regions.is_empty() {
        return Err("source project does not define any Region blocks".to_string());
    }
    if !config.game.start_region.trim().is_empty()
        && !project
            .regions
            .iter()
            .any(|region| region.map.name == config.game.start_region)
    {
        return Err(format!(
            "[game].start_region '{}' does not match any Region",
            config.game.start_region
        ));
    }
    if !config.game.start_screen.trim().is_empty()
        && !project
            .screens
            .values()
            .any(|screen| screen.map.name == config.game.start_screen)
    {
        return Err(format!(
            "[game].start_screen '{}' does not match any Screen",
            config.game.start_screen
        ));
    }

    normalize_project_modules(&mut project);

    Ok(project)
}

fn compile_region(
    source_region: SourceRegion,
    character_templates: &IndexMap<String, Uuid>,
    item_templates: &IndexMap<String, Uuid>,
    item_template_data: &IndexMap<Uuid, Item>,
    player_id: &str,
) -> Result<Region, String> {
    if source_region.terrain.is_empty() {
        return Err(format!(
            "Region '{}' is missing a terrain block",
            source_region.id
        ));
    }

    let mut region = Region::new();
    region.id = Uuid::new_v4();
    region.name = source_region.name.clone();
    region.module = module_shell("Region", &region.name, false);
    region.map = build_map(&source_region)?;
    region.characters.clear();
    region.items.clear();

    let spawns = collect_spawns(&source_region.terrain);
    for spawn in spawns {
        match spawn.kind {
            SpawnKind::Player => {
                let template_id = character_templates
                    .get(player_id)
                    .copied()
                    .ok_or_else(|| format!("player template '{}' was not generated", player_id))?;
                let instance = character_instance(player_id, template_id, spawn.x, spawn.y, true);
                region.characters.insert(instance.id, instance);
            }
            SpawnKind::Character(glyph) => {
                let id = glyph.to_string();
                if let Some(template_id) = character_templates.get(&id).copied() {
                    let instance = character_instance(&id, template_id, spawn.x, spawn.y, false);
                    region.characters.insert(instance.id, instance);
                }
            }
            SpawnKind::Item(glyph) => {
                let id = glyph.to_string();
                let item = if let Some(template_id) = item_templates.get(&id).copied() {
                    item_instance(template_id, item_template_data, spawn.x, spawn.y)
                } else {
                    default_item_instance(glyph, spawn.x, spawn.y)
                };
                region.items.insert(item.id, item);
            }
        }
    }

    Ok(region)
}

fn build_map(source_region: &SourceRegion) -> Result<Map, String> {
    let width = source_region
        .terrain
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let height = source_region.terrain.len();
    if width < 2 || height < 2 {
        return Err(format!(
            "Region '{}' terrain must be at least 2x2",
            source_region.id
        ));
    }

    let mut map = Map::default();
    map.name = source_region.id.clone();
    map.camera = MapCamera::TwoD;
    map.vertices.clear();
    map.linedefs.clear();
    map.sectors.clear();
    map.geometry_objects.clear();
    map.entities.clear();
    map.items.clear();

    let x0 = 0.0;
    let y0 = 0.0;
    let x1 = width as f32;
    let y1 = height as f32;
    let v0 = map.add_vertex_at(x0, y0);
    let v1 = map.add_vertex_at(x1, y0);
    let v2 = map.add_vertex_at(x1, y1);
    let v3 = map.add_vertex_at(x0, y1);
    map.create_linedef_manual(v0, v1);
    map.create_linedef_manual(v1, v2);
    map.create_linedef_manual(v2, v3);
    map.create_linedef_manual(v3, v0);
    let sector_id = map
        .close_polygon_manual()
        .ok_or_else(|| format!("failed to create sector for Region '{}'", source_region.id))?;
    if let Some(sector) = map.find_sector_mut(sector_id) {
        sector.name = source_region.name.clone();
        sector.properties.set(
            "data",
            Value::Str(format!(
                "title = \"{}\"\n",
                escape_toml_string(&source_region.name)
            )),
        );
        sector.properties.set(
            "eldiron_source_terrain",
            Value::Str(source_region.terrain.join("\n")),
        );
        sector.properties.set(
            "eldiron_source_default",
            Value::Str(source_region.default.clone()),
        );
        sector
            .properties
            .set("source", Value::Source(PixelSource::Off));
    }

    Ok(map)
}

fn compile_screen(
    source_screen: SourceScreen,
    viewport: &ViewportSection,
) -> Result<Screen, String> {
    if source_screen.widgets.is_empty() {
        return Err(format!(
            "Screen '{}' does not define any widgets",
            source_screen.id
        ));
    }

    let mut screen = Screen::new();
    screen.name = source_screen.name.clone();
    screen.map = build_screen_map(&source_screen, viewport)?;
    Ok(screen)
}

fn build_screen_map(
    source_screen: &SourceScreen,
    viewport: &ViewportSection,
) -> Result<Map, String> {
    let mut map = Map::default();
    map.name = source_screen.id.clone();
    map.camera = MapCamera::TwoD;
    map.grid_size = viewport.grid_size.max(1) as f32;
    map.vertices.clear();
    map.linedefs.clear();
    map.sectors.clear();
    map.geometry_objects.clear();
    map.entities.clear();
    map.items.clear();

    let start_x = -(viewport.width as f32) / 2.0;
    let start_y = -(viewport.height as f32) / 2.0;
    for widget in &source_screen.widgets {
        add_screen_widget_sector(&mut map, widget, start_x, start_y)?;
    }
    Ok(map)
}

fn add_screen_widget_sector(
    map: &mut Map,
    widget: &SourceWidget,
    start_x: f32,
    start_y: f32,
) -> Result<(), String> {
    if widget.width <= 0 || widget.height <= 0 {
        return Err(format!(
            "Widget '{}' must have positive width and height",
            widget.name
        ));
    }

    let x0 = start_x + widget.x as f32;
    let y0 = start_y + widget.y as f32;
    let x1 = x0 + widget.width as f32;
    let y1 = y0 + widget.height as f32;
    let v0 = map.add_vertex_at(x0, y0);
    let v1 = map.add_vertex_at(x1, y0);
    let v2 = map.add_vertex_at(x1, y1);
    let v3 = map.add_vertex_at(x0, y1);
    map.possible_polygon.clear();
    let linedefs = vec![
        map.create_linedef_manual(v0, v1),
        map.create_linedef_manual(v1, v2),
        map.create_linedef_manual(v2, v3),
        map.create_linedef_manual(v3, v0),
    ];
    map.possible_polygon.clear();
    let sector_id = map
        .find_free_sector_id()
        .ok_or_else(|| format!("failed to allocate screen widget '{}'", widget.name))?;
    for linedef_id in &linedefs {
        if let Some(linedef) = map
            .linedefs
            .iter_mut()
            .find(|linedef| linedef.id == *linedef_id)
            && !linedef.sector_ids.contains(&sector_id)
        {
            linedef.sector_ids.push(sector_id);
        }
    }
    map.sectors.push(Sector::new(sector_id, linedefs));
    if let Some(sector) = map.find_sector_mut(sector_id) {
        sector.name = widget.name.clone();
        sector
            .properties
            .set("data", Value::Str(widget_data(&widget.role, &widget.data)));
        sector
            .properties
            .set("source", Value::Source(PixelSource::Off));
    }
    Ok(())
}

fn widget_data(role: &str, data: &str) -> String {
    let data = data.trim();
    if data.is_empty() {
        return format!("[ui]\nrole = \"{}\"\n", escape_toml_string(role));
    }
    let Ok(mut table) = data.parse::<toml::Table>() else {
        return data.to_string();
    };
    let ui = table
        .entry("ui")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if let Some(ui) = ui.as_table_mut() {
        ui.entry("role")
            .or_insert_with(|| toml::Value::String(role.to_string()));
    }
    toml::to_string(&table).unwrap_or_else(|_| data.to_string())
}

fn parse_source(src: &str) -> Result<SourceDocument, String> {
    let mut document = SourceDocument::default();
    for block in find_named_blocks(src, "Character")? {
        document.characters.push(parse_character(&block)?);
    }
    for block in find_named_blocks(src, "Item")? {
        document.items.push(parse_item(&block)?);
    }
    for block in find_named_blocks(src, "Region")? {
        document.regions.push(parse_region(&block)?);
    }
    for block in find_named_blocks(src, "Screen")? {
        document.screens.push(parse_screen(&block)?);
    }
    Ok(document)
}

fn parse_character(block: &NamedBlock) -> Result<SourceCharacter, String> {
    let name = string_field(&block.body, "name").unwrap_or_else(|| title_case_id(&block.name));
    let glyph = string_field(&block.body, "glyph").and_then(|value| value.chars().next());
    let data = brace_block(&block.body, "data")
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    let script = brace_block(&block.body, "script")
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    Ok(SourceCharacter {
        id: block.name.clone(),
        name,
        glyph,
        data,
        script,
    })
}

fn parse_item(block: &NamedBlock) -> Result<SourceItem, String> {
    let name = string_field(&block.body, "name").unwrap_or_else(|| title_case_id(&block.name));
    let glyph = string_field(&block.body, "glyph").and_then(|value| value.chars().next());
    let data = brace_block(&block.body, "data")
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    let script = brace_block(&block.body, "script")
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    Ok(SourceItem {
        id: block.name.clone(),
        name,
        glyph,
        data,
        script,
    })
}

fn parse_region(block: &NamedBlock) -> Result<SourceRegion, String> {
    let name = string_field(&block.body, "name").unwrap_or_else(|| title_case_id(&block.name));
    let default = bare_field(&block.body, "default").unwrap_or_else(|| "wall.stone".to_string());
    let terrain = triple_string_field(&block.body, "terrain")
        .ok_or_else(|| format!("Region '{}' is missing terrain \"\"\"...\"\"\"", block.name))?;
    let lines = normalize_terrain_lines(&terrain);
    Ok(SourceRegion {
        id: block.name.clone(),
        name,
        default,
        terrain: lines,
    })
}

fn parse_screen(block: &NamedBlock) -> Result<SourceScreen, String> {
    let name = string_field(&block.body, "name").unwrap_or_else(|| title_case_id(&block.name));
    let mut widgets = Vec::new();
    for widget in find_named_blocks(&block.body, "widget")? {
        widgets.push(parse_widget(&widget)?);
    }
    for widget in find_named_blocks(&block.body, "Widget")? {
        widgets.push(parse_widget(&widget)?);
    }
    Ok(SourceScreen {
        id: block.name.clone(),
        name,
        widgets,
    })
}

fn parse_widget(block: &NamedBlock) -> Result<SourceWidget, String> {
    let role = string_field(&block.body, "role").unwrap_or_else(|| "none".to_string());
    let x = int_field(&block.body, "x").unwrap_or(0);
    let y = int_field(&block.body, "y").unwrap_or(0);
    let width = int_field(&block.body, "width")
        .or_else(|| int_field(&block.body, "w"))
        .ok_or_else(|| format!("widget '{}' is missing width", block.name))?;
    let height = int_field(&block.body, "height")
        .or_else(|| int_field(&block.body, "h"))
        .ok_or_else(|| format!("widget '{}' is missing height", block.name))?;
    let data = brace_block(&block.body, "data")
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    Ok(SourceWidget {
        name: block.name.clone(),
        role,
        x,
        y,
        width,
        height,
        data,
    })
}

#[derive(Debug)]
struct NamedBlock {
    name: String,
    body: String,
}

fn find_named_blocks(src: &str, keyword: &str) -> Result<Vec<NamedBlock>, String> {
    let mut blocks = Vec::new();
    let mut index = 0;
    while let Some(relative) = src[index..].find(keyword) {
        let start = index + relative;
        if !is_boundary(src, start, keyword.len()) {
            index = start + keyword.len();
            continue;
        }
        let mut cursor = start + keyword.len();
        cursor = skip_ws(src, cursor);
        let (name, after_name) = parse_quoted(src, cursor)
            .ok_or_else(|| format!("{keyword} block at byte {start} is missing a quoted name"))?;
        cursor = skip_ws(src, after_name);
        if src[cursor..].chars().next() != Some('{') {
            return Err(format!("{keyword} \"{name}\" is missing '{{'"));
        }
        let end = find_matching_brace(src, cursor)
            .ok_or_else(|| format!("{keyword} \"{name}\" has an unterminated body"))?;
        blocks.push(NamedBlock {
            name,
            body: src[cursor + 1..end].to_string(),
        });
        index = end + 1;
    }
    Ok(blocks)
}

fn is_boundary(src: &str, start: usize, len: usize) -> bool {
    let before = src[..start].chars().next_back();
    let after = src[start + len..].chars().next();
    !before.is_some_and(is_ident_char) && !after.is_some_and(is_ident_char)
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn skip_ws(src: &str, mut index: usize) -> usize {
    while let Some(ch) = src[index..].chars().next() {
        if !ch.is_whitespace() {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

fn parse_quoted(src: &str, start: usize) -> Option<(String, usize)> {
    if src[start..].chars().next()? != '"' {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    let mut index = start + 1;
    for ch in src[index..].chars() {
        index += ch.len_utf8();
        if escaped {
            out.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some((out, index));
        } else {
            out.push(ch);
        }
    }
    None
}

fn find_matching_brace(src: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut index = open;
    while index < src.len() {
        if !in_string && src[index..].starts_with("\"\"\"") {
            index += 3;
            let end = src[index..].find("\"\"\"")?;
            index += end + 3;
            continue;
        }
        let ch = src[index..].chars().next()?;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += ch.len_utf8();
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += ch.len_utf8();
    }
    None
}

fn string_field(src: &str, key: &str) -> Option<String> {
    let pos = find_field_pos(src, key)?;
    let mut cursor = skip_ws(src, pos + key.len());
    if src[cursor..].chars().next()? != '=' {
        return None;
    }
    cursor = skip_ws(src, cursor + 1);
    parse_quoted(src, cursor).map(|(value, _)| value)
}

fn bare_field(src: &str, key: &str) -> Option<String> {
    let pos = find_field_pos(src, key)?;
    let mut cursor = skip_ws(src, pos + key.len());
    if src[cursor..].chars().next()? != '=' {
        return None;
    }
    cursor = skip_ws(src, cursor + 1);
    let rest = &src[cursor..];
    let end = rest
        .find(|ch: char| ch == '\n' || ch == '\r' || ch == '}')
        .unwrap_or(rest.len());
    Some(rest[..end].trim().trim_matches('"').to_string())
}

fn int_field(src: &str, key: &str) -> Option<i32> {
    bare_field(src, key)?.parse().ok()
}

fn triple_string_field(src: &str, key: &str) -> Option<String> {
    let pos = find_field_pos(src, key)?;
    let mut cursor = skip_ws(src, pos + key.len());
    let at_pos = src[cursor..].starts_with("at ");
    if at_pos {
        let rest = &src[cursor..];
        cursor += rest.find("\"\"\"")?;
    }
    if !src[cursor..].starts_with("\"\"\"") {
        cursor = skip_ws(src, cursor);
        if !src[cursor..].starts_with("\"\"\"") {
            return None;
        }
    }
    let content_start = cursor + 3;
    let content_end = src[content_start..].find("\"\"\"")? + content_start;
    Some(
        src[content_start..content_end]
            .trim_matches('\n')
            .to_string(),
    )
}

fn brace_block<'a>(src: &'a str, key: &str) -> Option<&'a str> {
    let pos = find_field_pos(src, key)?;
    let cursor = skip_ws(src, pos + key.len());
    if src[cursor..].chars().next()? != '{' {
        return None;
    }
    let end = find_matching_brace(src, cursor)?;
    Some(&src[cursor + 1..end])
}

fn find_field_pos(src: &str, key: &str) -> Option<usize> {
    let mut search_from = 0;
    while let Some(relative) = src[search_from..].find(key) {
        let pos = search_from + relative;
        if is_boundary(src, pos, key.len()) {
            return Some(pos);
        }
        search_from = pos + key.len();
    }
    None
}

fn normalize_terrain_lines(src: &str) -> Vec<String> {
    let lines: Vec<&str> = src
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .collect();
    let indent = lines
        .iter()
        .map(|line| line.chars().take_while(|ch| ch.is_whitespace()).count())
        .min()
        .unwrap_or(0);
    lines
        .into_iter()
        .map(|line| line.chars().skip(indent).collect())
        .collect()
}

fn collect_spawns(lines: &[String]) -> Vec<Spawn> {
    let mut spawns = Vec::new();
    for (y, line) in lines.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            let kind = if ch == '@' {
                Some(SpawnKind::Player)
            } else if ch.is_ascii_uppercase() {
                Some(SpawnKind::Character(ch))
            } else if ch.is_ascii_lowercase() {
                Some(SpawnKind::Item(ch))
            } else {
                None
            };
            if let Some(kind) = kind {
                spawns.push(Spawn { kind, x, y });
            }
        }
    }
    spawns
}

fn ensure_player_template(
    project: &mut Project,
    character_templates: &mut IndexMap<String, Uuid>,
    player_id: &str,
) {
    if character_templates.contains_key(player_id) {
        return;
    }
    let mut character = Character::new();
    character.name = title_case_id(player_id);
    character.data = character_data(player_id, player_id);
    character.authoring = character_authoring(Some('@'));
    character.module = module_shell("CharacterTemplate", &character.name, true);
    let template_id = character.id;
    character_templates.insert(player_id.to_string(), template_id);
    project.characters.insert(template_id, character);
}

fn character_instance(id: &str, template_id: Uuid, x: usize, y: usize, player: bool) -> Character {
    let mut character = Character::new();
    character.name = title_case_id(id);
    character.position = grid_position(x, y);
    character.character_id = template_id;
    character.data = character_data(id, if player { id } else { "" });
    character.module = module_shell("CharacterTemplate", &character.name, player);
    character
}

fn item_instance(
    template_id: Uuid,
    item_template_data: &IndexMap<Uuid, Item>,
    x: usize,
    y: usize,
) -> Item {
    let mut item = Item::new();
    if let Some(template) = item_template_data.get(&template_id) {
        item.name = template.name.clone();
        item.data = template.data.clone();
        item.source = template.source.clone();
        item.source_debug = template.source_debug.clone();
        item.authoring = template.authoring.clone();
        item.module = template.module.clone();
    }
    item.position = grid_position(x, y);
    item.item_id = template_id;
    if item.module.is_null() {
        item.module = module_shell("ItemTemplate", &item.name, false);
    }
    item
}

fn default_item_instance(glyph: char, x: usize, y: usize) -> Item {
    let mut item = Item::new();
    item.name = default_item_name(glyph);
    item.position = grid_position(x, y);
    item.data = item_data(&glyph.to_string(), Some(glyph));
    item.authoring = item_authoring(Some(glyph));
    item.module = module_shell("ItemTemplate", &item.name, false);
    item
}

fn module_shell(module_type: &str, name: &str, player: bool) -> serde_json::Value {
    serde_json::json!({
        "filter_text": "",
        "id": Uuid::nil().to_string(),
        "module_type": module_type,
        "name": name,
        "player": player,
        "routines": {},
        "view_name": "",
    })
}

fn normalize_project_modules(project: &mut Project) {
    if project.world_module.is_null() {
        project.world_module = module_shell("World", "", false);
    }

    for item in project.items.values_mut() {
        if item.module.is_null() {
            item.module = module_shell("ItemTemplate", &item.name, false);
        }
    }
    for character in project.characters.values_mut() {
        if character.module.is_null() {
            character.module = module_shell(
                "CharacterTemplate",
                &character.name,
                character
                    .data
                    .parse::<toml::Table>()
                    .ok()
                    .and_then(|data| {
                        data.get("attributes")
                            .and_then(toml::Value::as_table)
                            .cloned()
                    })
                    .and_then(|attributes| attributes.get("player").and_then(toml::Value::as_bool))
                    .unwrap_or(false),
            );
        }
    }

    for region in &mut project.regions {
        if region.module.is_null() {
            region.module = module_shell("Region", &region.name, false);
        }
        for item in region.items.values_mut() {
            if item.module.is_null() {
                item.module = module_shell("ItemTemplate", &item.name, false);
            }
        }
        for character in region.characters.values_mut() {
            if character.module.is_null() {
                character.module = module_shell(
                    "CharacterTemplate",
                    &character.name,
                    character.data.contains("player = true"),
                );
            }
        }
    }
}

fn register_ruleset_item_glyphs(
    items: &IndexMap<Uuid, Item>,
    item_templates: &mut IndexMap<String, Uuid>,
) {
    for (glyph, ruleset_id) in [('h', "wild_herb"), ('b', "blessed_herb")] {
        if let Some(template_id) = ruleset_item_template_id(items, ruleset_id) {
            item_templates.insert(glyph.to_string(), template_id);
        }
    }
}

fn ruleset_item_template_id(items: &IndexMap<Uuid, Item>, ruleset_id: &str) -> Option<Uuid> {
    items.iter().find_map(|(id, item)| {
        item.data
            .parse::<toml::Table>()
            .ok()
            .and_then(|data| {
                data.get("attributes")
                    .and_then(toml::Value::as_table)
                    .cloned()
            })
            .and_then(|attributes| {
                let id_matches = attributes
                    .get("ruleset_id")
                    .and_then(toml::Value::as_str)
                    .is_some_and(|value| value == ruleset_id);
                id_matches.then_some(*id)
            })
    })
}

fn grid_position(x: usize, y: usize) -> Vec3<f32> {
    Vec3::new(x as f32 + 0.5, 1.0, y as f32 + 0.5)
}

fn project_config(
    game: &GameSection,
    viewport: &ViewportSection,
    terminal: &TerminalSection,
) -> String {
    format!(
        "[game]\nstart_region = \"{}\"\nstart_screen = \"{}\"\nterminal_mode = \"{}\"\nsimulation_mode = \"{}\"\nturn_timeout_ms = {}\nauto_create_player = {}\ncollision_mode = \"{}\"\n\n[viewport]\nwidth = {}\nheight = {}\ngrid_size = {}\nunit = \"{}\"\nresize = \"{}\"\n\n[terminal]\ntext_updates = {}\n",
        escape_toml_string(&game.start_region),
        escape_toml_string(&game.start_screen),
        escape_toml_string(&game.terminal_mode),
        escape_toml_string(&game.simulation_mode),
        game.turn_timeout_ms,
        game.auto_create_player,
        escape_toml_string(&game.collision_mode),
        viewport.width,
        viewport.height,
        viewport.grid_size,
        escape_toml_string(&viewport.unit),
        escape_toml_string(&viewport.resize),
        terminal.text_updates
    )
}

fn character_data(id: &str, player_id: &str) -> String {
    let is_player = !player_id.is_empty() && id == player_id;
    let mut data = format!(
        "[attributes]\nplayer = {}\nsource_id = \"{}\"\n",
        is_player,
        escape_toml_string(id)
    );
    if is_player {
        data.push_str(
            "\n[input]\nw = \"control.forward\"\na = \"control.left\"\ns = \"control.backward\"\nd = \"control.right\"\nup = \"control.forward\"\nleft = \"control.left\"\ndown = \"control.backward\"\nright = \"control.right\"\n",
        );
        data.push_str("g = \"intent(take)\"\n");
    }
    data
}

fn character_authoring(glyph: Option<char>) -> String {
    match glyph {
        Some(glyph) => format!(
            "[terminal]\nglyph = \"{}\"\n",
            escape_toml_string(&glyph.to_string())
        ),
        None => String::new(),
    }
}

fn item_data(id: &str, glyph: Option<char>) -> String {
    let glyph = glyph
        .map(|glyph| glyph.to_string())
        .unwrap_or_else(|| id.to_string());
    format!(
        "[attributes]\nsource_id = \"{}\"\nsource_symbol = \"{}\"\nterminal_glyph = \"{}\"\non_take = \"take\"\n",
        escape_toml_string(id),
        escape_toml_string(&glyph),
        escape_toml_string(&glyph)
    )
}

fn item_authoring(glyph: Option<char>) -> String {
    match glyph {
        Some(glyph) => format!(
            "[terminal]\nglyph = \"{}\"\n",
            escape_toml_string(&glyph.to_string())
        ),
        None => String::new(),
    }
}

fn default_item_name(glyph: char) -> String {
    match glyph {
        'h' => "Herb".to_string(),
        'b' => "Blessed Herb".to_string(),
        _ => format!("Item {glyph}"),
    }
}

fn title_case_id(id: &str) -> String {
    id.split(['_', '-', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn default_main_source() -> String {
    "main.els".to_string()
}

fn default_player() -> String {
    "player".to_string()
}

fn default_terminal_mode() -> String {
    "roguelike".to_string()
}

fn default_terminal_text_updates() -> bool {
    true
}

fn default_simulation_mode() -> String {
    "hybrid".to_string()
}

fn default_turn_timeout_ms() -> u32 {
    600
}

fn default_collision_mode() -> String {
    "tile".to_string()
}

fn default_viewport_width() -> u32 {
    80
}

fn default_viewport_height() -> u32 {
    24
}

fn default_viewport_grid_size() -> u32 {
    40
}

fn default_viewport_unit() -> String {
    "cell".to_string()
}

fn default_viewport_resize() -> String {
    "fit".to_string()
}

fn default_output() -> String {
    "dist/game.eldiron".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_source() {
        let source = r#"
Character "player" {
  name = "Player"
  glyph = "@"
  script {
    on_interact {
      say("hello");
    }
  }
}

Item "herb" {
  name = "Herb"
  glyph = "h"
}

Region "cellar" {
  default = wall.stone
  terrain """
  ####
  #@.#
  ####
  """
}

Screen "play" {
  widget "Game" {
    role = "game"
    width = 80
    height = 20
  }
}
"#;
        let parsed = parse_source(source).expect("source parses");
        assert_eq!(parsed.characters.len(), 1);
        assert_eq!(parsed.characters[0].id, "player");
        assert!(parsed.characters[0].script.contains("on_interact"));
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].id, "herb");
        assert_eq!(parsed.regions.len(), 1);
        assert_eq!(parsed.regions[0].terrain.len(), 3);
        assert_eq!(parsed.screens.len(), 1);
        assert_eq!(parsed.screens[0].widgets.len(), 1);
        assert_eq!(parsed.screens[0].widgets[0].role, "game");
    }

    #[test]
    fn compiles_minimal_project() {
        let config = ProjectToml {
            project: ProjectSection {
                name: "Test".to_string(),
            },
            source: SourceSection::default(),
            game: GameSection {
                start_region: "cellar".to_string(),
                start_screen: String::new(),
                terminal_mode: "text".to_string(),
                simulation_mode: "hybrid".to_string(),
                turn_timeout_ms: 600,
                collision_mode: "tile".to_string(),
                auto_create_player: true,
                player: "player".to_string(),
            },
            viewport: ViewportSection::default(),
            terminal: TerminalSection::default(),
            build: BuildSection::default(),
        };
        let source = SourceDocument {
            characters: Vec::new(),
            items: Vec::new(),
            regions: vec![SourceRegion {
                id: "cellar".to_string(),
                name: "Cellar".to_string(),
                default: "wall.stone".to_string(),
                terrain: vec!["###".to_string(), "#@#".to_string(), "###".to_string()],
            }],
            screens: Vec::new(),
        };
        let project = compile_project(&config, source).expect("project compiles");
        assert_eq!(project.regions.len(), 1);
        assert_eq!(project.regions[0].map.name, "cellar");
        assert_eq!(project.characters.len(), 1);
        assert_eq!(project.regions[0].characters.len(), 1);
    }
}
