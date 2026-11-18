const RENDER_KEYS: &[&str] = &[
    "ambient_color",
    "ambient_strength",
    "ao_radius",
    "ao_samples",
    "avatar_highlight_enabled",
    "avatar_highlight_fill",
    "avatar_highlight_lift",
    "avatar_highlight_rim",
    "bump_strength",
    "fade_mode",
    "firstp_blur_far",
    "firstp_blur_near",
    "fog_color",
    "fog_density",
    "lighting_model",
    "max_shadow_distance",
    "max_shadow_steps",
    "max_sky_distance",
    "max_transparency_bounces",
    "ms_per_frame",
    "msaa_samples",
    "point_shadow_softness",
    "reflection_samples",
    "shadow_bias",
    "shadow_enabled",
    "shadow_resolution",
    "shadow_strength",
    "sky_color",
    "style",
    "sun_color",
    "sun_direction",
    "sun_enabled",
    "sun_intensity",
];
fn table(source: &str) -> Result<toml::Table, String> {
    if source.trim().is_empty() {
        Ok(toml::Table::new())
    } else {
        source.parse::<toml::Table>().map_err(|e| e.to_string())
    }
}
fn select(source: &toml::Table, keys: &[&str]) -> toml::Table {
    source
        .iter()
        .filter(|(k, _)| keys.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}
fn encode(t: toml::Table) -> Result<String, String> {
    toml::to_string(&t).map_err(|e| e.to_string())
}
pub fn config(source: &str) -> Result<String, String> {
    let source = table(source)?;
    let mut output = toml::Table::new();
    for (section, keys) in [
        ("render", RENDER_KEYS),
        (
            "post",
            &[
                "enabled",
                "tone_mapper",
                "exposure",
                "saturation",
                "luminance",
                "gamma",
                "grit",
                "posterize",
                "palette_bias",
                "shadow_lift",
                "edge_soften",
            ][..],
        ),
        (
            "simulation",
            &[
                "enabled",
                "night_sky_color",
                "morning_sky_color",
                "midday_sky_color",
                "evening_sky_color",
                "night_sun_color",
                "morning_sun_color",
                "midday_sun_color",
                "evening_sun_color",
                "sunrise_time",
                "sunset_time",
                "color_transition_duration_hours",
            ][..],
        ),
        ("materials", &["background_2d", "pixels_per_tile"][..]),
        ("appearance", &["sprite_pixels_2d", "pixel_size_3d"][..]),
        (
            "viewport",
            &[
                "width",
                "height",
                "grid_size",
                "unit",
                "upscale",
                "window_scale",
                "cursor_id",
                "background_color_2d",
                "visibility_range_2d",
                "visibility_alpha_2d",
                "screen_background",
            ][..],
        ),
        (
            "game",
            &[
                "mode",
                "locale",
                "target_fps",
                "game_tick_ms",
                "ticks_per_minute",
                "start_screen",
                "play_screen",
                "firstp_eye_level",
                "avatar_shading",
                "avatar_skin_auto_shading",
                "collision_mode",
                "movement_units_per_sec",
                "turn_speed_deg_per_sec",
                "entity_block_mode",
                "persistent_intents",
                "click_intents_2d",
                "persistent_2d_intents",
                "base_currency_name",
                "base_currency_symbol",
                "currencies",
                "gear_slots",
                "weapon_slots",
                "health",
            ][..],
        ),
        (
            "multiplayer",
            &["endpoint", "deployment_id", "allow_insecure_loopback"][..],
        ),
    ] {
        if let Some(t) = source.get(section).and_then(toml::Value::as_table) {
            output.insert(section.into(), select(t, keys).into());
        }
    }
    output
        .entry("game".to_string())
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .unwrap()
        .insert("mode".into(), "multiplayer".into());
    encode(output)
}
pub fn screen_settings(source: &str) -> Result<String, String> {
    let source = table(source)?;
    let mut out = toml::Table::new();
    if let Some(layout) = source.get("layout").and_then(toml::Value::as_table) {
        out.insert(
            "layout".into(),
            select(
                layout,
                &[
                    "mode",
                    "width",
                    "height",
                    "anchor",
                    "reference_width",
                    "reference_height",
                ],
            )
            .into(),
        );
    }
    encode(out)
}
pub fn ui_data(source: &str) -> Result<String, String> {
    // Match screen compilation: legacy invalid widget drafts are ignored.
    // Do not publish their unparsed text, which can contain unrelated source.
    let source = table(source).unwrap_or_default();
    let mut out = toml::Table::new();
    if let Some(ui) = source.get("ui") {
        out.insert("ui".into(), ui.clone());
    }
    encode(out)
}
pub fn template_data(source: &str) -> Result<String, String> {
    let source = table(source)?;
    let mut out = toml::Table::new();
    if let Some(attributes) = source.get("attributes").and_then(toml::Value::as_table) {
        out.insert(
            "attributes".into(),
            select(
                attributes,
                &[
                    "avatar",
                    "avatar_id",
                    "appearance_profile",
                    "appearance_geometry",
                    "appearance_worn_geometry",
                    "appearance_materials",
                    "appearance_padding",
                    "source",
                    "source_off",
                    "tile_id",
                    "icon",
                    "icon_off",
                    "icon_source",
                    "icon_color",
                    "icon_role",
                    "model",
                    "model_id",
                    "sprite",
                    "scale",
                    "radius",
                    "height",
                    "layer",
                    "size",
                    "alignment",
                    "item_type",
                    "ruleset_id",
                    "ruleset_path",
                ],
            )
            .into(),
        );
    }
    encode(out)
}
const UI_ROW_KEYS: &[&str] = &[
    "name",
    "label",
    "description",
    "short",
    "icon",
    "icon_off",
    "type",
    "format",
    "unit",
    "role",
    "min",
    "max",
    "default",
    "visible",
    "visibility",
    "category",
    "order",
    "group",
    "color",
    "ui",
    "command",
    "intent",
    "skill",
    "required_skill",
    "consumes",
    "target",
    "range",
    "cooldown",
    "cost",
    "costs",
    "requires",
    "requirements",
    "known",
    "tags",
    "aliases",
    "needs_target",
    "ability",
    "spell",
    "show_in_action_bar",
    "show_in_ui",
    "action_bar",
    "unlocks",
    "progression",
    "spells",
    "abilities",
    "traits",
    "skills",
    "value",
    "capacity",
    "weight",
    "avatar_anchors",
    "weapon_categories",
    "default_avatar",
    "kind",
    "precision",
    "symbol",
    "items",
    "ingredients",
    "outputs",
];
pub fn rules(source: &str) -> Result<String, String> {
    let source = table(source)?;
    let mut out = toml::Table::new();
    for key in ["ui", "visuals"] {
        if let Some(v) = source.get(key) {
            out.insert(key.into(), v.clone());
        }
    }
    if let Some(defaults) = source
        .get("identity")
        .and_then(|v| v.get("defaults"))
        .and_then(toml::Value::as_table)
    {
        let mut identity = toml::Table::new();
        identity.insert(
            "defaults".into(),
            select(defaults, &["race", "class", "profession"]).into(),
        );
        out.insert("identity".into(), identity.into());
    }
    for key in [
        "attributes",
        "classes",
        "races",
        "actions",
        "abilities",
        "spells",
        "traits",
        "conditions",
        "skills",
        "currencies",
        "recipes",
        "equipment",
        "intents",
    ] {
        if let Some(rows) = source.get(key).and_then(toml::Value::as_table) {
            let mut public = toml::Table::new();
            for (id, row) in rows {
                if let Some(row) = row.as_table() {
                    let keys = if key == "attributes" && id == "roles" {
                        &[
                            "health",
                            "max_health",
                            "mana",
                            "max_mana",
                            "stamina",
                            "max_stamina",
                            "level",
                            "experience",
                            "armor",
                            "weapon_damage",
                        ][..]
                    } else {
                        UI_ROW_KEYS
                    };
                    if id != "defaults" {
                        public.insert(id.clone(), select(row, keys).into());
                    }
                } else if (key == "equipment" && UI_ROW_KEYS.contains(&id.as_str()))
                    || (key == "attributes"
                        && ["primary", "resources", "combat", "progression"].contains(&id.as_str()))
                {
                    public.insert(id.clone(), row.clone());
                }
            }
            out.insert(key.into(), public.into());
        }
    }
    encode(out)
}

pub fn region_config(source: &str) -> Result<String, String> {
    let source = table(source)?;
    let mut out = toml::Table::new();
    if let Some(render) = source.get("render").and_then(toml::Value::as_table) {
        out.insert("render".into(), select(render, RENDER_KEYS).into());
    }
    encode(out)
}
