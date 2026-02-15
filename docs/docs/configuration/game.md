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
# AO samples (number of rays, default 2)
ao_samples = 1.0

# AO radius (default 0.5)
ao_radius = 0.5

# Reflection samples (0 = disabled, >=1 = GGX PBR reflection rays)
reflection_samples = 0.0

# Bump strength (0.0-1.0, default 1.0)
bump_strength = 1.0

# Max transparency bounces (default 8)
max_transparency_bounces = 8.0

# Max shadow distance (default 10.0)
max_shadow_distance = 10.0

# Max sky distance (default 50.0)
max_sky_distance = 50.0

# Max shadow steps for transparent shadows (0.0 = no transparent shadows, default: 0)
max_shadow_steps = 0.0

# Static sky color (used when simulation is disabled)
sky_color = "#87CEEB"

# Static sun color (used when simulation is disabled)
sun_color = "#FFFACD"

# Sun intensity/brightness multiplier
sun_intensity = 1.0

# Static sun direction as [x, y, z] (used when simulation is disabled)
sun_direction = [-0.5, -1.0, -0.3]

# Enable/disable sun lighting
sun_enabled = true

# Ambient light color
ambient_color = "#999999"

# Ambient light strength (0.0 - 1.0)
ambient_strength = 0.3

# Fog color
fog_color = "#808080"

# Fog density (0.0 = no fog, higher values = denser fog)
fog_density = 0.0
```

### **Option Descriptions**

All `[render]` options apply **only to the 3D renderer** (they do not affect the 2D tile/sprite renderer).

- **`ao_samples`** — Number of ambient-occlusion rays. Higher counts smooth AO but cost performance; set `0` to disable.
- **`ao_radius`** — World-space distance for AO rays. Larger radii darken wider cavities; very large values can over-darken scenes.
- **`reflection_samples`** — GGX PBR reflection rays per hit. `0` disables ray-traced reflections; ≥1 enables glossy/mirror reflections.
- **`bump_strength`** — Scales normal-map/bump detail (0–1). Lower to flatten surfaces; `1.0` keeps full normal-map effect.
- **`max_transparency_bounces`** — Max number of ray marches through transparent layers (glass, billboards). Raise for stacked glass; lower for speed.
- **`max_shadow_distance`** — Furthest distance to search for shadow casters for sun/point lights. Reduce to speed up or limit long shadows.
- **`max_sky_distance`** — Maximum trace distance for sky/environment contribution and reflections. Lower to clip sky on dense scenes.
- **`max_shadow_steps`** — Steps for transparency-aware shadows. `0` = fast binary shadows; >0 enables softer/transparent shadowing at higher cost.
- **`sky_color`** — Static sky RGB used when sky simulation is off or beyond `max_sky_distance`.
- **`sun_color`** — Static sun RGB tint used when sun simulation is off.
- **`sun_intensity`** — Multiplier for sun brightness. Increase for harsher lighting; reduce for softer daylight.
- **`sun_direction`** — Sun vector `[x, y, z]` (points from light to scene). Adjust to change time-of-day lighting angle.
- **`sun_enabled`** — Toggles the directional sun light. Set `false` for indoor or emissive-only scenes.
- **`ambient_color`** — Uniform ambient RGB independent of sky. Use to fill shadows with a specific hue.
- **`ambient_strength`** — Scalar (0–1) for ambient_color energy. Higher values lighten occluded areas.
- **`fog_color`** — Fog RGB tint applied with distance-based fog.
- **`fog_density`** — Strength of exponential-squared fog. `0` disables; higher values increase haze with distance.

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
-
