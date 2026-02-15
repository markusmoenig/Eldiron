---
title: "Client Commands"
sidebar_position: 8
---

Client side scripting happens in the `user_event` section of scripts (and only for player characters). You can delete the `user_event` section for non player characters.

:::note
Player characters must be marked with `player = true` in their data to receive input events.
:::

## Usage

Example `user_event()` script for handling key presses:

```eldrin
fn user_event(event, value) {
    if event == "key_down" {
        let key = value;
        if key == "w" {
            action( "forward");
        }
        if key == "a" {
            action( "left");
        }
        if key == "d" {
            action( "right");
        }
        if key == "s" {
            action( "backward");
        }
        if key == "u" {
            intent( "use");
        }
        if key == "t" {
            intent( "attack");
        }
        if key == "k" {
            intent( "take");
        }
    }
    if event == "key_up" {
        let key = value;
        action( "none");
    }
}
```

---

## `action`

Triggers a movement or rotation action for a player character. This command is typically used in response to user input inside the `user_event()` method of a player-controlled character.

---

## Action Types

The `action()` command accepts the following action types:

### `forward`

```eldrin
action("forward")
```

- **2D / Isometric**: Move the player north.
- **First-Person**: Move the player forward in their current facing direction.

---

### `left`

```eldrin
action("left")
```

- **2D / Isometric**: Move the player west.
- **First-Person**: Rotate the player to their left.

---

### `right`

```eldrin
action("right")
```

- **2D / Isometric**: Move the player east.
- **First-Person**: Rotate the player to their right.

---

### `backward`

```eldrin
action("backward")
```

- **2D / Isometric**: Move the player south.
- **First-Person**: Move the player backward in their current facing direction.

---

### `none`

```eldrin
action("none")
```

Stops any ongoing movement or rotation.

---

## `intent`

Handling differs by game widget:

- In 2D mode, it tells the server that the next movement action or click is intent-based rather than movement-based. Because 2D defaults to movement, you can set an intent in a specific direction for the next movement action.
- In 3D mode, it sets the current intent to the new value until you change it.

You can send any string with this command as long as you handle it inside the [intent event](events#intent) for characters or items.

The `intent` command in `user_events` is useful for shortcuts; for user interfaces use [button widgets](/docs/screens/screens_widgets#button-widgets).

---
