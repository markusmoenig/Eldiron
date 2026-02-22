---
title: "Game Configuration"
sidebar_position: 1
---

You can configure **Eldiron** by selecting the **Game -> Settings** item in the **project tree**.

---

## Game Configuration

Game configuration options are located in the `[game]` section.

```toml
[game]
target_fps = 30                # The target frames per second for the game.
game_tick_ms = 250             # The milliseconds per game tick.
ticks_per_minute = 4           # The amount of ticks per in-game minute.
movement_units_per_sec = 4     # Base movement speed in world units per second.
turn_speed_deg_per_sec = 120   # First-person turn speed in degrees per second.
firstp_eye_level = 1.7         # First-person camera eye height above the entity base Y.
entity_block_mode = "always"   # The block mode, "always" or "never".
auto_create_player = true      # Whether to auto-create a player entity.
start_region = ""              # The name of the region to start the game in.
start_screen = ""              # The name of the screen to show at startup.

# Base currency configuration
base_currency_name = "Gold"      # Display name of the primary in-game currency.
base_currency_symbol = "G"       # Symbol used to represent the currency (e.g. "G" for Gold).

# The supported gear slots
gear_slots = ["legs", "head", "torso"]

# The supported weapon slots
weapon_slots = ["main_hand", "off_hand"]

# The attribute which handles health & death
health = "HP"

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

- **`gear_slots`**
  The **valid gear slots** of items. Items can define it's gear slot by setting `slot` in the data tool.

- **`weapon_slots`**
  The **valid weapon slots** of items. Items can define it's weapon slot by setting `slot` in the data tool.

### `health`

  The name of the health attribute for characters. When smaller or equal to zero means the character is considered **dead** and it's [mode](/docs/characters_items/attributes#mode) attribute is set to '"dead", this is handled automatically by [took_damage](/docs/characters_items/server_commands#took_damage) . If you want to use another attribute name than change the default **"HP"** value to something else.

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

:::tip
The supported game configuration options will increase over time.
:::

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
grid_size = 32      # Size of one grid tile in pixels.
upscale = "aspect"  # 'aspect' upscales the game output to the screen dimensions. 'none' otherwise.
cursor_id = "..."   # The tile id of the default mouse cursor.
```

### **Option Descriptions**

- **`width` / `height`**
    Defines the **starting resolution** of the game window or screen.
    You can adjust these values to target common resolutions like 1280×720 or 1920×1080.

- **`grid_size`**
    Sets the **pixel size of a single tile** in the world/grid.
    This affects rendering and snapping behavior in tools and the viewport layout.

- **`upscale`**
    If set to **"aspect"** upscales the game output to the screen / window resolution keeping the viewport aspect-ratio intact.
    **"none"** (the default) does not upscale and centers the output.

- **`cursor_id`**
    The [tile id](/docs/what_is/#tileid) for the default mouse cursor.
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
