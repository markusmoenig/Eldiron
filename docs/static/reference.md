
This chapter serves as a reference for **scripting and data attributes** used in [Characters](/docs/creator/characters) and [Items](/docs/creator/items) in Eldiron.

Since many **events, commands, and attributes** are shared between the two, they are **listed together**, with any **specific differences noted where applicable**.

:::warning
This chapter is work in progress.
:::

---

## Setting Basic Attributes

Attributes can be set using **Python (Code Tool)** or **TOML (Data Tool)**.

### **Using Python (Code Tool)**

The attributes can be set inside the templates or the [instance scripts](/docs/creator/characters/#instances) of **characters** or **items**.

```python
# Give the character or item a name (if they differ from the template)
set_attr("name", "Golden Key")

# Set the tile ID for the character or item. Get the tile ID from the tile-picker.
set_tile("tile_id")

# Make the character or item visible / invisible
set_attr("visible", False)

# Change the collision radius for characters and items (default is 0.5)
set_attr("radius", 0.3)

# Item specific

# Make the item blocking (based on its radius)
set_attr("blocking", True)

# Make the item static (doors, campfires etc.). Static items cannot be picked up.
set_attr("static", True)

# Setting general purpose attributes
set_attr("STR", 10)
```

### **Using TOML (Data Tool)**

```toml
[attributes]
# General purpose attributes. By convention use uppercase for character attributes.
STR = 10

# Give the character or item a name (if they differ from the template)
name = "Orc"

# Set the tile ID for the character or item. Get the tile ID from the tile-picker
tile_id = "tile_id"

# Make the character or item visible / invisible
visible = false

# Change the collision radius for characters and items (default is 0.5)
radius = 0.3

# Character specific

# Register the character as a player character which receives user events
player = true

# Item specific

# Defines the slot of the item (if any) when equipped.
slot = "legs"

# If the item overrides colors of the character when equipped, specify it here.
color = "#ff0000"

# When the item is equipped, specifies the names of sectors whose colors should be overriden with the above color.
# This is useful when you dont want to override the geometry but just the color of a character geometries nodegraph.
color_targets = ["left_leg", "right_leg"]

# When the item is equipped, specifies the names of linedefs this item geometry should be attached to. If 'geo_targets' is not
# present Eldiron checks if there is a linedef with a name equal to this item's slot name and uses that. So use 'geo_targets' only
# if you want to attach the item geometry to several linedefs.
geo_targets = ["left_shoulder", "right_shoulder"]

# Make the item blocking (based on its radius).
blocking = true

# Make the item static (doors, campfires etc.). Static items cannot be picked up.
static = true

# The worth of the item in the base currency. This is its trade value.
worth = 0.0

# This item represents money. A monetary item will not be picked up by itself but its worth is added
# to the entities wallet.
monetary = false

# Defines the amount of default inventory slots (how many items the character can carry). If not specified
# the amount of slots is set to 0 (i.e. the character is unable to take any items).
inventory_slots = 8
```

---

## Available Scripting Commands

### Commands for Both Characters and Items

These commands can be used for both **characters** and **items**:

```python
# Block the listed events from being send to the character or item for the given amount
# of in-game minutes.
block_events(minutes, "event1", "event2",...)

# Deal damage to the given entity or item identified by its ID.
# Damage is a Python array of information which gets send to the receiver via an
# `take_damage` event.
# Example: deal_damage(id, {"physical": 10})
# Send all relevant data to the receiver who can calculate the final damage and apply it.
deal_damage(entity_id | item_id, damage)

# Send a debug message to the Log.
debug(arg1, arg2, ...)

# Get an attribute of the current character or item.
get_attr("key")

# Get an attribute of the given character.
get_entity_attr(entity_id, "key")

# Get an attribute of the given item.
get_item_attr(item_id,"key")

# Returns an array of filtered item ids of the given character's inventory.
# Returns all items if filter_string is empty. Otherwise, returns items whose name
# or class names contain the filter_string.
inventory_items_of(entity_id, filter_string)

# Return a list of entity ids within the radius of the character or item.
# This has many use cases, like a door checking if it can close as no players overlap.
entities_in_radius()

# Return the name of the sector the character or item is in.
get_sector_name()

# Send the event string to the character or item after a given amount of in-game minutes.
# By default, one in game minute is one second in real time.
notify_in(minutes, event_string)

# Set an attribute of the current character or item. Value can be any Python value.
set_attr("key", value)

# Enables / disable entity proximity tracking. If enabled, the entity or item will receive
# "proximity_warning" events with a list of entity ids within the radius.
# Works similarly to entities_in_radius(), but auto generates events.
# Use with get_entity_attr() to check for entities to take action on (attack, heal, talk etc).
set_proximity_tracking(True / False, radius)

# Send a message to the given character. Category is optional and used for coloring
# messages in the message widget (see Screens & Widgets).
message(entity_id, message, category)
```

### Commands for Characters Only

These commands are **exclusive to characters**:

```python

# Creates a new item of the given class name and adds it to the character's inventory. It returns the id of the created
# item (in case you want to equp it) or -1 on failure.
add_item(class_name)

# Returns an array of filtered item ids in the character's inventory.
# Returns all items if filter_string is empty. Otherwise, returns items whose name
# or class names contain the filter_string.
inventory_items(filter_string)

# Drops items in the character's inventory.
# Drops all items if filter_string is empty. Otherwise, drops items whose name
# or class names contain the filter_string.
drop_items(filter_string)

# Loop: Walks the character in a random direction for the given distance and speed.
# After arrival, sleeps for a random amount of in-game-minutes between max_sleep / 2 and max_sleep.
# Example: random_walk(1.0, 1.0, 8)
# Mostly used for NPCs
random_walk(distance, speed, max_sleep)

# Loop: Walks the character in a random direction in the current sector for the given distance and speed.
# In between sleeps the character for a random amount of in-game-minutes between max_sleep / 2 and max_sleep.
# This command is useful for NPCs that need to move around randomly without leaving a sector (shop etc.)
# Example: random_walk_in_sector(1.0, 8)
# Mostly used for NPCs
random_walk_in_sector(distance, speed, max_sleep)

# Take an item from the region. The item is removed from the region and added to the character's inventory. Returns
# True if successful or False otherwise (inventory is full).
take(item_id)

# Equips the item (weapon or gear) with the given item_id to its slot. If there is an existing item in that slot it will
# be unequipped and put back into the inventory. This function returns True on success and False if it fails (for example)
# when the item has no slot name.
equip(item_id)
```

### Commands for Items Only

These commands are **exclusive to items**:

```python
# None yet
```

### Applying Player Actions

The `action` command is used to trigger **player actions** based on user input.

By default, a character’s `user_event` method looks like this:

```python
def user_event(self, event, value):
    if event == 'key_down':
        if value == 'w':
            action("forward")
        if value == 'a':
            action("left")
        if value == 'd':
            action("right")
        if value == 's':
            action("backward")
    if event == 'key_up':
            action("none")
```

:::note
Player characters must be registered using the `player = true` in the characters data.
:::

:::tip
These movement commands are **camera-independent** and work for **2D, isometric, and first-person cameras**.
:::

### Available Actions

```python
# 2D and Isometric: Move the player north.
# First-Person: Move the player forward in their current facing direction.
action("forward")

# 2D and Isometric: Move the player west.
# First-Person: Rotate the player to their left.
action("left")

# 2D and Isometric: Move the player east.
# First-Person: Rotate the player to their right.
action("right")

# 2D and Isometric: Move the player south.
# First-Person: Move the player backward in their current facing direction.
action("backward")

# Stop any movement / action.
action("none")
```

## Events

This is a list of events, categorized into **System Events** (sent to `event()`) and **User Events** (sent to `user_event()`).

### System Events

| Event Name             | Value           | Description |
|------------------------|----------------|-------------|
| **`startup`**            | *(None)*        | Called when the entity or item is created. |
| **`bumped_into_entity`** | `entity_id` *(int)* | Triggered when an entity bumps into another entity. |
| **`bumped_into_item`**   | `item` *(int)*   | Triggered when an entity bumps into an item. |
| **`bumped_by_entity`**   | `entity_id` *(int)* | Triggered when another entity collides with this entity or item. |
| **`clicked`**            | *dict* `{ entity_id, distance }` | Triggered when the player clicks on an entity or item. Includes the clicking entity's ID and distance. |
| **`take_damage`**        | *dict*           | Triggered by the `deal_damage` command. The dictionary contains the data passed to `deal_damage()`. |

### User Events

| Event Name      | Value      | Description |
|----------------|-----------|-------------|
| **`key_down`** | *(string)* | Triggered when a key is pressed. The event sends the value of the pressed key (e.g., `"w"`, `"a"`, `"space"`). |
| **`key_up`**   | *(string)* | Triggered when a key is released. The event sends the value of the released key (e.g., `"w"`, `"a"`, `"space"`). |
