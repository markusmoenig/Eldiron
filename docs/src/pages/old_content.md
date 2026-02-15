## Behavior

After creating a character and activating the **Code Tool**, you will see an **Eldrin Script** that defines the character’s behavior.

```python
class NewCharacter:

    def event(self, event, value):
        pass

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

If you rename the **class** (default: `NewCharacter`), the **character template name** will update automatically in the Character section.

:::tip
The **Python class name** is also the **character template name**.
:::

---

## Events

Eldiron uses **events** to trigger actions.

Events are categorized into:
- **System Events** – Triggered by the game engine.
- **User Events** – Triggered by the player.

System events call the `event` method, while user events call the `user_event` method.

### System Events

Example: The `startup` event is called when a character is created. The `bump_item` event is triggered when a character collides with an item.

```python
class Player:

    def event(self, event, value):
        if event == 'startup':
            set_attr("STR", 10)
```

In this example:
- The **STR (Strength) attribute** is set to **10**.

### User Events

The `user_event` method is only needed for **player characters** and can be omitted for **NPCs**.

In the earlier example, `user_event` handles **keyboard input**, allowing the player to move using the **WASD keys**.

---

# Instances

When you **drag a character template into the map**, it creates a **new instance**.

The [Character](/docs/creator/sections/#region) section lists all **character instances** in the **region**. Characters are displayed with a **human avatar** on the map.

:::tip
**Click & Drag** a character in the map to move it.
**Press 'Delete'** to remove a character instance.
:::

### Instance Initialization

When a **character instance** is selected in the **Region** section, the **Code Tool** will display its **instance initialization script**:

```python
def setup():
    """Initialize the character instance"""
    pass
```

This script allows **instance-specific** configurations.

The `setup` method in the **template** applies to **all characters** of that type, while the **instance script** allows customization of **individual characters**.

### Example: Creating a Stronger Orc

You have a general **`Orc` template**, but you want a **stronger Orc** guarding a chest.

```python
def setup():
    set_attr("STR", 15)
```

This makes **only this Orc instance** stronger by setting its **Strength (STR) to 15**.

---

# Data Tool

The **Data Tool** allows you to edit the **initial attributes** of **character instances**.

Example **TOML configuration**:

```toml
[attributes]
STR = 10

# The character is visible on the map
visible = true
# The radius of the character's collision circle
radius = 0.5
```

In this example:
- `"STR"` is set to **10** (same as in the **Code Tool**).
- `"visible = true"` ensures the character **appears on the map**.
- `"radius = 0.5"` defines the **collision area**.

Using the **Data Tool** is **often more convenient** than setting attributes in **code**, especially for common properties.

## Learn More

See the **[Scripting & Data Reference](/docs/scripting_data/reference)** for a complete list of available **events, commands, actions, and data properties**.

## Opening a Door

Opening a door can be achieved in different ways depending on your **gameplay mechanics**. For example, you could open a door when:

- A character **bumps into it**.
- The player **clicks a "Use" or "Open" button**.

Let's start with the **simplest approach**: opening a door when a character **bumps into it**.

### **Example: Auto-Opening Door**

```python
# Taken from https://github.com/markusmoenig/Eldiron/blob/master/examples/Harbor.eldiron

class Door:

    def event(self, event, value):

        if event == "bumped_by_entity":
            set_attr("visible", False)
            set_attr("blocking", False)
            notify_in(2, "close_door")

        if event == "close_door":
            if len(entities_in_radius()) == 0:
                set_attr("visible", True)
                set_attr("blocking", True)
            else:
                notify_in(2, "close_door")
```

### **How It Works**

1. When a character **bumps into the door**, the `bumped_by_entity` event is triggered.
2. The door **opens** by setting:
   - `visible = False` (door disappears).
   - `blocking = False` (door no longer blocks movement).
3. The `notify_in(2, "close_door")` function **delays closing** the door for **2 seconds**.
4. When `close_door` is triggered, the script:
   - **Checks if the area is empty** (`entities_in_radius() == 0`).
   - If **empty**, the door **closes** (`visible = True`, `blocking = True`).
   - If **not empty**, it **delays the closing** again by 2 seconds.

:::note
Setting the `blocking` attribute isn’t strictly necessary in this case, since the door **instantly opens** on contact. However, it is included here for **clarity and flexibility** in different scenarios.
:::

## Opening a Locked Gate

Opening a **locked gate** works similarly to opening a **door**, but with one key difference:

- The gate **requires a key** to open.
- On the `bumped_by_entity` event, the script checks if the **character has the correct key** in their inventory.

```python
# Taken from https://github.com/markusmoenig/Eldiron/blob/master/examples/Harbor.eldiron

class Gate:

    def event(self, event, value):
        if event == "bumped_by_entity":
            if len(inventory_items_of(value, "Golden Key")) > 0:
                set_attr("visible", False)
                set_attr("blocking", False)
                notify_in(2, "close_gate")
        if event == "close_gate":
            if len(entities_in_radius()) == 0:
                set_attr("visible", True)
                set_attr("blocking", True)
            else:
                notify_in(2, "close_gate")
```

### **How It Works**

1. When a character **bumps into the gate**, the `bumped_by_entity` event is triggered.
2. The `value` parameter contains the **ID of the character** that bumped into the gate.
3. The script checks if that character **has a "Golden Key"** in their inventory using:
   - `inventory_items_of(value, "Golden Key") > 0`
4. If the character **has the key**, the gate **opens** by setting:
   - `visible = False` (gate disappears).
   - `blocking = False` (gate no longer blocks movement).
5. The `notify_in(2, "close_gate")` function **delays closing** the gate for **2 seconds**.
6. When `close_gate` is triggered, the script:
   - **Checks if the area is empty** (`entities_in_radius() == 0`).
   - If **empty**, the gate **closes** (`visible = True`, `blocking = True`).
   - If **not empty**, it **delays closing** again by **2 seconds**.

:::tip
The `value` parameter in `bumped_by_entity` holds the **ID of the character** who collided with the gate.
We pass this ID to `inventory_items_of(value, "Golden Key")` to **check that specific character's inventory**.
This script can be adapted to check for **different key types** by replacing `"Golden Key"` with another item name.
