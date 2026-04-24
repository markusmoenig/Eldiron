---
title: "Game Configuration"
sidebar_position: 1
---

You can configure **Eldiron** by selecting the **Game -> Settings** item in the **project tree**.

Other game-level TOML documents are edited separately:

- **Game -> Authoring**: [Authoring Configuration](./authoring)
- **Game -> World / Visual Scripting**: global graph-based runtime logic
- **Game -> World / Eldrin Scripting**: global text-based runtime logic
- **Game -> Rules**: [Rules](../rules)
- **Game -> Locales**: [Localization](../localization)
- **Game -> Audio FX**: [Audio](../audio)

For party-bound UI such as portraits, equipped hand slots, or member-specific inventory widgets, see [Screen Widgets](../screens/widgets) and the character [Attributes](../characters_items/attributes).

---

## Game Configuration

Game configuration options are located in the `[game]` section.

```toml
[game]
target_fps = 30                # The target frames per second for the game.
game_tick_ms = 250             # The milliseconds per game tick.
simulation_mode = "realtime"   # Gameplay pacing: "realtime", "turn_based", or "hybrid".
turn_timeout_ms = 600          # In hybrid mode, advance one gameplay step after this idle timeout.
ticks_per_minute = 4           # The amount of ticks per in-game minute.
movement_units_per_sec = 4     # Base movement speed in world units per second.
turn_speed_deg_per_sec = 120   # First-person turn speed in degrees per second.
firstp_eye_level = 1.7         # First-person camera eye height above the entity base Y.
entity_block_mode = "always"   # The block mode, "always" or "never".
collision_mode = "tile"        # Collision/path mode: "tile" or "mesh".
auto_create_player = true      # Whether to auto-create a player entity.
start_region = ""              # The name of the region to start the game in.
start_screen = ""              # The name of the screen to show at startup.
click_intents_2d = false       # Target 2D intents with mouse clicks while keeping WASD movement.
auto_walk_2d = false           # In walk mode, clicking terrain in 2D makes the player path-walk there.

# Base currency configuration
base_currency_name = "Gold"      # Display name of the primary in-game currency.
base_currency_symbol = "G"       # Symbol used to represent the currency (e.g. "G" for Gold).
locale = "en"                    # Active locale used for localized strings and rules-based messages.

# The supported gear slots
gear_slots = ["legs", "head", "torso"]

# The supported weapon slots
weapon_slots = ["main_hand", "off_hand"]

# The attribute which handles health & death
health = "HP"

# The attribute which stores the current level
level = "LEVEL"

# The attribute which stores accumulated experience
experience = "EXP"

# Enables generated marker ramp shading for avatars.
avatar_shading = true

# Enables generated marker ramp shading for skin markers.
avatar_skin_auto_shading = false
```

### **Option Descriptions**

- **`target_fps`**
  - Defines the **refresh rate** of the game.
  - A **higher FPS** results in **smoother gameplay**, but increases CPU usage.

- **`game_tick_ms`**
  - Sets the **milliseconds per game tick**, which is **Eldiron’s internal clock**.
  - Events, actions, and player interactions are processed **each tick**.
  - Default: `250 ms`, meaning **4 ticks per second** (suitable for most games).
  - In `turn_based` and `hybrid` simulation modes, this also defines the size of one discrete gameplay step.

- **`simulation_mode`**
  - Controls how gameplay simulation advances.
  - `"realtime"` → Current Eldiron behavior. Gameplay advances continuously with wall-clock time.
  - `"turn_based"` → Gameplay advances only when the player commits an action.
  - `"hybrid"` → Gameplay advances when the player commits an action, or automatically after `turn_timeout_ms` of inactivity.
  - Rendering and UI stay realtime in all modes; this setting gates gameplay progression, not screen refresh.
  - Default: `"realtime"`.

- **`turn_timeout_ms`**
  - Idle timeout used by `simulation_mode = "hybrid"`.
  - When the player does nothing for this long, Eldiron advances one discrete gameplay step automatically.
  - Ignored in `realtime` and `turn_based`.
  - Default: `600`.

- **`ticks_per_minute`**
  - Defines the **number of ticks per in-game minute**.
  - Default: `4`, meaning **1 in-game minute = 1 real-time second**.
  - To sync in-game time with real time, set this value to **`60 * 4 = 240`**.

- **`movement_units_per_sec`**
  - Defines the base movement speed in world units per second.
  - Other movement actions scale relative to this value.
  - Default: `4`.

- **`turn_speed_deg_per_sec`**
  - Defines first-person turning speed in degrees per second.
  - Helps tune camera yaw feel without changing movement speed.
  - Default: `120`.

- **`firstp_eye_level`**
  - Defines first-person camera eye height above the entity's base Y position.
  - Applied on the client in first-person mode only.
  - Default: `1.7`.

- **`entity_block_mode`**
  - Controls whether **entities (i.e., characters)** can move through each other.
  - `"always"` → Entities **always block each other**.
  - `"never"` → Entities **never block each other**.

- **`collision_mode`**
  - Selects which collision/pathfinding mode movement actions use (`move`, `goto`, `close_in`).
  - `"tile"` → Use tile/linedef-based collision and pathing.
  - `"mesh"` → Use 3D mesh/chunk-based collision and pathing.
  - Default: `"tile"`.
  - Recommendation: use `"tile"` for 2D games and `"mesh"` for 3D games.

- **`auto_create_player`**
  If `true`, Eldiron will automatically **create a player instance** in the map if one is defined.
  Useful for quickly testing and building games without needing to implement a full character creation process.
  If `false`, the player must be created manually—typically using a **screen** and **user input flow**.

- **`start_region`**
  The **name of the region** the game will load when it starts.
  If `start_screen` is not set, this first region will be shown by default.

- **`start_screen`**
  The **name of the screen** to load on startup.
  If empty, Eldiron will display a black screen.

- **`click_intents_2d`**
  - Enables click-targeted intents in `2d` and `2d_grid` input modes.
  - When enabled, selecting an intent changes mouse interaction, but movement keys still walk as usual.
  - Click a target to apply the current intent; the selected intent stays active until you switch it.
  - Intent hover / clicked cursors also apply in 2D while this mode is active.
  - Default: `false`.

- **`auto_walk_2d`**
  - Enables terrain click-to-walk for `2d` and `2d_grid` input modes.
  - Only applies when no intent is active, so walk mode stays separate from click-targeted intents.
  - Uses the normal runtime `Goto` pathing action.
  - Default: `false`.

- **`gear_slots`**
  The **valid gear slots** of items. Items can define it's gear slot by setting `slot` in the data tool.

- **`weapon_slots`**
  The **valid weapon slots** of items. Items can define it's weapon slot by setting `slot` in the data tool.

### `health`

  The name of the health attribute for characters. When it becomes smaller or equal to zero, the character is considered **dead** and its [mode](/docs/characters_items/attributes#mode) attribute is set to `"dead"` automatically by the server damage system. If you want to use another attribute name, change the default **"HP"** value to something else.

### `level`

  The name of the character attribute used as the current level for [Rules](../rules) progression formulas. Default: **"LEVEL"**.

### `experience`

  The name of the character attribute used to store accumulated experience. Default: **"EXP"**.

### `avatar_shading`

- Enables generated runtime marker-ramp shading for avatars.
- Default: `true`.
- Set to `false` to keep avatar marker colors flat (no auto ramp shading).

### `avatar_skin_auto_shading`

- Enables generated runtime marker-ramp shading for skin markers.
- Default: `false`.
- Useful when skin already contains artist-authored light/dark tones and should stay flat.

#### `base_currency_name`

- The **display name** of your game's primary currency (e.g. `"Gold"`, `"Credits"`).
- Used in the UI, item pricing, and trade.

#### `base_currency_symbol`

- The **short symbol** shown with currency values (e.g. `"G"`).
- Appears alongside numbers (e.g. `50 G`, `100 💎`).

#### `locale`

- Selects the active locale table, for example `"en"` or `"de"`.
- Use `"auto"` to follow the system locale.
- The actual translation tables are edited separately in **Game / Locales**.

---

## Say Configuration

Speech bubble options are located in the `[say]` section.

```toml
[say]
duration = 1                 # How long say bubbles stay visible (in in-game minutes).
default = "#E5E501"          # Default text color when category is empty or unknown.

background_enabled = true    # Whether to draw a background behind say text.
background_color = "#00000080" # Background RGBA color (#RRGGBBAA), includes alpha.

# Optional per-category text colors:
npc = "#FFFFFF"
warning = "#FF6666"
quest = "#66CCFF"
```

### Option Descriptions

- **`duration`**
  - Lifetime of a `say(...)` bubble in **in-game minutes**.
  - Accepts integer or float values (for example `1` or `1.5`).

- **`default`**
  - Fallback text color if no category is set in `say(message, category)` or if the category is not found.

- **`background_enabled`**
  - Enables/disables the speech bubble background rectangle.

- **`background_color`**
  - Background color including alpha channel (`#RRGGBBAA`), e.g. `#00000080` for 50% black.

- **Category keys (e.g. `npc`, `warning`, `quest`)**
  - Any additional key in `[say]` is treated as a text color category.
  - Use it from scripting with `say("Text", "category_name")`.

### **Using In-Game Time for Events**

Some commands use **in-game minutes** for timing.
For example, the `notify_in` command schedules events **after a set number of in-game minutes**:

```python
notify_in(2, "close_door")
```

With the **default settings**, this means the event will trigger **after 2 real-time seconds**.

---

## Viewport Configuration

Viewport configuration defines the resolution and grid used when the game starts.

```toml
[viewport]
width = 1280        # Width of the game viewport in pixels.
height = 720        # Height of the game viewport in pixels.
window_scale = 1.0  # Multiplies startup window size (e.g. 2.0 = 2x bigger window).
grid_size = 32      # Size of one grid tile in pixels.
upscale = "aspect"  # 'aspect' upscales the game output to the screen dimensions. 'none' otherwise.
background_color_2d = "#000000" # 2D viewport background color.
visibility_range_2d = 0 # 2D visible range around the player in tiles. 0 disables the limit.
visibility_alpha_2d = 0.82 # LOS mask blend toward the background color (0..1).
screen_background = "#000000" # Screen/widget background color.
cursor_id = "..."   # The tile id of the default mouse cursor.
target_rect_color = "" # Optional target rectangle color for the current leader target in 2D.
```

### **Option Descriptions**

- **`width` / `height`**
    Defines the **starting resolution** of the game window or screen.
    You can adjust these values to target common resolutions like 1280×720 or 1920×1080.

- **`window_scale`**
    Multiplies the startup window size while keeping the game viewport resolution unchanged.
    For example, with `width = 1280`, `height = 720`, and `window_scale = 2.0`, the window opens at 2560×1440.
    Default is `1.0`.

- **`grid_size`**
    Sets the **pixel size of a single tile** in the world/grid.
    This affects rendering and snapping behavior in tools and the viewport layout.

- **`upscale`**
    If set to **"aspect"** upscales the game output to the screen / window resolution keeping the viewport aspect-ratio intact.
    **"none"** (the default) does not upscale and centers the output.

- **`background_color_2d`**
    Background color used for the 2D game viewport and the editor's 2D game preview.
    Accepts `#RRGGBB` or `#RRGGBBAA`.

- **`visibility_range_2d`**
    Limits how many tiles are visible around the player in 2D.
    The mask uses the 2D background color outside the visible range.
    Use `0` or a negative value to disable the limit.

- **`visibility_alpha_2d`**
    Controls how strongly hidden 2D tiles blend toward `background_color_2d`.
    `0` keeps the original color, `1` fully hides the tile.

- **`screen_background`**
    Background color used when rendering screens and screen widgets.
    If omitted, it falls back to `background_color_2d`.

- **`cursor_id`**
    The [tile id](/docs/what_is/#tileid) for the default mouse cursor.

- **`target_rect_color`**
    Optional rectangle color for drawing the current leader target in 2D game widgets.
    Uses `target` / `attack_target` from the current leader entity.
    Leave empty to disable.
---

## Render Configuration

Render configuration options are located in the `[render]` section.

```toml
[render]
# Static sky color (used when simulation is disabled).
sky_color = "#87CEEB"

# Static sun color (used when simulation is disabled).
sun_color = "#FFFACD"

# Sun intensity/brightness multiplier.
sun_intensity = 1.0

# Static sun direction as [x, y, z] (used when simulation is disabled).
sun_direction = [-0.5, -1.0, -0.3]

# Enable/disable sun lighting.
sun_enabled = true

# Ambient light color.
ambient_color = "#999999"

# Ambient light strength (0.0 - 1.0).
ambient_strength = 0.3

# Fog color.
fog_color = "#808080"

# Fog density (0.0 = no fog, higher values = denser fog).
fog_density = 0.0

# Shadow toggle.
shadow_enabled = true

# Shadow strength (default 0.8).
shadow_strength = 0.8

# Shadow-map resolution.
shadow_resolution = 1024

# Shadow depth bias.
shadow_bias = 0.0015

# Fade mode for alpha/visibility transitions: "ordered_dither" or "uniform".
fade_mode = "ordered_dither"

# Lighting model: "lambert", "cook_torrance", or "pbr".
lighting_model = "cook_torrance"

# Avatar readability boost toggle for Raster 3D avatars.
avatar_highlight_enabled = true

# Avatar readability lift multiplier (1.0 = unchanged).
avatar_highlight_lift = 1.12

# Avatar ambient fill contribution.
avatar_highlight_fill = 0.20

# Avatar rim-light contribution.
avatar_highlight_rim = 0.18

# Bump mapping strength (0.0 = off, 1.0 = full).
bump_strength = 1.0

# MSAA sample count for raster 3D (0 = off, 4 = on).
msaa_samples = 4

# First-person blur transition start distance.
firstp_blur_near = 3.0

# First-person blur transition end distance.
firstp_blur_far = 8.0
```

### **Option Descriptions**

All `[render]` options apply **only to the 3D renderer** (they do not affect the 2D tile/sprite renderer).

- **`sky_color`** — Static sky RGB used when sky simulation is off.
- **`sun_color`** — Static sun RGB tint used when sun simulation is off.
- **`sun_intensity`** — Multiplier for sun brightness. Increase for harsher lighting; reduce for softer daylight.
- **`sun_direction`** — Sun vector `[x, y, z]` (points from light to scene). Adjust to change time-of-day lighting angle.
- **`sun_enabled`** — Toggles the directional sun light. Set `false` for indoor or emissive-only scenes.
- **`ambient_color`** — Uniform ambient RGB independent of sky. Use to fill shadows with a specific hue.
- **`ambient_strength`** — Scalar (0–1) for ambient_color energy. Higher values lighten occluded areas.
- **`fog_color`** — Fog RGB tint applied with distance-based fog.
- **`fog_density`** — Strength of exponential-squared fog. `0` disables; higher values increase haze with distance.
- **`shadow_enabled`** — Enables or disables sun shadow-map rendering.
- **`shadow_strength`** — Shadow contribution amount (0–1). Lower values make shadows softer/fainter.
- **`shadow_resolution`** — Shadow-map size in pixels. Higher values sharpen shadows but increase GPU cost.
- **`shadow_bias`** — Depth bias used to reduce shadow acne/peter-panning.
- **`fade_mode`** — Visibility fade style for hidden/fading geometry (`ordered_dither` or `uniform`).
- **`lighting_model`** — Surface lighting model (`lambert`, `cook_torrance`, `pbr`).
- **`avatar_highlight_enabled`** — Enables avatar readability boost in Raster 3D.
- **`avatar_highlight_lift`** — Multiplier for avatar lit color (`1.0` = unchanged).
- **`avatar_highlight_fill`** — Extra ambient/albedo fill added to avatars.
- **`avatar_highlight_rim`** — Rim-light intensity for avatar silhouettes at grazing view angles.
- **`bump_strength`** — Scales normal-map/bump detail (0–1). Lower to flatten surfaces; `1.0` keeps full effect.
- **`msaa_samples`** — Raster 3D multisampling level (`0` = off, `4` = on) for edge anti-aliasing.
- **`firstp_blur_near`** — Distance where first-person texture blur transition starts.
- **`firstp_blur_far`** — Distance where first-person texture blur transition is fully applied.

---

## Post Configuration

Post configuration options are located in the `[post]` section.

```toml
[post]
# Enable/disable final post pass.
enabled = true

# Tone mapper: "none", "reinhard", "aces".
tone_mapper = "reinhard"

# Exposure multiplier before tone mapping.
exposure = 1.0

# Post saturation (1.0 = unchanged, 0.0 = grayscale).
saturation = 1.0

# Post luminance/brightness multiplier.
luminance = 1.0

# Output gamma.
gamma = 2.2
```

### **Option Descriptions**

- **`enabled`** — Enables or disables the post-processing stage.
- **`tone_mapper`** — Tone mapping operator used before gamma (`none`, `reinhard`, `aces`).
- **`exposure`** — Brightness multiplier applied before tone mapping.
- **`saturation`** — Color saturation multiplier; `0` is grayscale, `1` keeps original saturation.
- **`luminance`** — Overall post brightness multiplier.
- **`gamma`** — Final output gamma value.

---

## Runtime Overrides

`[render]`, `[post]`, and `[viewport]` define the authored defaults of your project.

At runtime, scripts can override these values through **world** and **region** contexts:

- `world.render.*`
- `region.render.*`
- `world.post.*`
- `region.post.*`

The final value is resolved in this order:

1. project configuration default
2. world script override
3. region script override

This lets you keep stable defaults in TOML and then change mood, fog, palette remap, or post processing dynamically during play.

### Palette Remap

The 2D palette remap lives under `render.pal.*`:

```eldrin
let world.render.pal.start = 0;
let world.render.pal.end = 9;
let world.render.pal.mode = "nearest";
let world.render.pal.blend = 1.0;
```

Supported fields:

- `render.pal.start`
- `render.pal.end`
- `render.pal.mode`
- `render.pal.blend`

Supported modes:

- `"disabled"`
- `"luma_ramp"`
- `"nearest"`
- `"dithered_ramp"`

### Render Overrides

Common runtime render overrides include:

```eldrin
let region.render.background_color_2d = "#272744";
let region.render.visibility_range_2d = 6.0;
let region.render.visibility_alpha_2d = 0.6;
let region.render.fog_color = "#20242c";
let region.render.fog_density = 5.0;
let world.render.sun_enabled = false;
```

Most scalar values from `[render]` are available at runtime, including:

- `background_color_2d`
- `visibility_range_2d`
- `visibility_alpha_2d`
- `sky_color`
- `sun_color`
- `sun_intensity`
- `sun_direction`
- `sun_enabled`
- `ambient_color`
- `ambient_strength`
- `fog_color`
- `fog_density`
- `shadow_enabled`
- `shadow_strength`
- `shadow_resolution`
- `shadow_bias`
- `fade_mode`
- `lighting_model`
- `avatar_highlight_enabled`
- `avatar_highlight_lift`
- `avatar_highlight_fill`
- `avatar_highlight_rim`
- `avatar_shading_enabled`
- `avatar_skin_shading_enabled`
- `ao_samples`
- `ao_radius`
- `bump_strength`
- `msaa_samples`
- `max_transparency_bounces`
- `max_shadow_distance`
- `max_sky_distance`
- `max_shadow_steps`
- `reflection_samples`
- `firstp_blur_near`
- `firstp_blur_far`
- `ms_per_frame`

### Post Overrides

The main post controls are also available at runtime:

```eldrin
let world.post.enabled = true;
let world.post.tone_mapper = "aces";
let world.post.exposure = 0.9;
let world.post.saturation = 0.7;
```

Supported runtime post fields:

- `post.enabled`
- `post.tone_mapper`
- `post.exposure`
- `post.gamma`
- `post.saturation`
- `post.luminance`

---

## Simulation Configuration

Simulation configuration options are located in the `[simulation]` section.

```toml
[simulation]
# Enable procedural daylight simulation (overrides static sky_color, sun_color, sun_direction)
enabled = true

# Sky color at night (dark)
night_sky_color = "#050510"

# Sky color at sunrise/sunset (morning)
morning_sky_color = "#FF9966"

# Sky color at midday
midday_sky_color = "#87CEEB"

# Sky color in the evening
evening_sky_color = "#FF8040"

# Sun/moon color at night (very dim)
night_sun_color = "#1A1A26"

# Sun color at sunrise/sunset (morning)
morning_sun_color = "#FFCC99"

# Sun color at midday
midday_sun_color = "#FFFFF2"

# Sun color in the evening
evening_sun_color = "#FFB380"

# Sunrise time in 24-hour format (e.g., 6.5 = 6:30 AM)
sunrise_time = 6.0

# Sunset time in 24-hour format (e.g., 18.5 = 6:30 PM)
sunset_time = 18.0

# Per-transition blend duration in in-game hours (0.5 = 30 minutes)
color_transition_duration_hours = 0.5
```

### **Option Descriptions**

These simulation values drive the **3D** AND **2D** procedural sky/sun lighting.

- **`enabled`** — Turns procedural daylight on. When `true`, it overrides static `sky_color`, `sun_color`, and `sun_direction` from `[render]`.
- **`night_sky_color`** — Sky tint used from sunset to sunrise.
- **`morning_sky_color`** — Sky tint blended in during the sunrise transition.
- **`midday_sky_color`** — Sky tint applied around noon for clear daylight.
- **`evening_sky_color`** — Sky tint used during sunset.
- **`night_sun_color`** — Dim sun/moon color at night for subtle skylight.
- **`morning_sun_color`** — Sun tint during sunrise; often warmer/orange.
- **`midday_sun_color`** — Sun tint at noon; typically neutral/white.
- **`evening_sun_color`** — Sun tint during sunset; typically warm.
- **`sunrise_time`** — 24-hour decimal time when sunrise starts (e.g., `6.5` = 06:30). Drives interpolation from night → morning.
- **`sunset_time`** — 24-hour decimal time when sunset starts (e.g., `18.5` = 18:30). Drives interpolation from midday → evening → night.
- **`color_transition_duration_hours`** — Duration of each day/night color blend window in in-game hours. Example: `0.5` = 30 minutes, `0.25` = 15 minutes.
