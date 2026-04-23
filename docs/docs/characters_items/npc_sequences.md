---
title: "NPC Sequences"
sidebar_position: 8
---

## Overview

NPC sequences are **background behavior** for characters.

Use them for planned actions such as:

- walk to work
- open a door
- walk through
- close the door
- wait at a counter
- go home in the evening

Sequences do **not** replace the normal event system.

Use the event system for reactive behavior:

- `talk`
- `attack`
- `use`
- `entered`
- `left`
- `time`

Use sequences for long-running intent.

---

## How It Fits Together

A practical model is:

- **Events** decide what should happen right now.
- **Sequences** describe what the NPC was doing in the background.
- **Scripts** choose whether to keep the sequence running, pause it, resume it, or cancel it.

Typical pattern:

1. A `time` event starts a background sequence.
2. A `talk` or `attack` event interrupts that behavior if needed.
3. The script decides whether to call `pause_sequence()`, `resume_sequence()`, or `cancel_sequence()`.

This keeps reactive gameplay in `event(...)`, while the actual route/workflow stays in data.

---

## Defining Sequences

Sequences are defined in the character **Attributes** editor as TOML.

```toml
[attributes]
timeout = 10

[behavior.sequences.go_to_work]

[[behavior.sequences.go_to_work.steps]]
action = "goto"
target = "DoorOutside"
speed = 1.0

[[behavior.sequences.go_to_work.steps]]
action = "use"
target = "FrontDoor"

[[behavior.sequences.go_to_work.steps]]
action = "goto"
target = "OfficeInside"
speed = 1.0

[[behavior.sequences.go_to_work.steps]]
action = "use"
target = "FrontDoor"

[[behavior.sequences.go_to_work.steps]]
action = "goto"
target = "Desk"
speed = 1.0
```

Supported step actions in v1:

- `goto`
- `use`
- `wait`

### `goto`

Moves to the nearest named sector matching `target`.

```toml
[[behavior.sequences.go_to_work.steps]]
action = "goto"
target = "Desk"
speed = 1.0
```

### `use`

Triggers a normal `use` intent against the nearest matching item or entity.

```toml
[[behavior.sequences.go_to_work.steps]]
action = "use"
target = "FrontDoor"
```

This goes through the same runtime interaction path as player/item `use`, rather than bypassing it.

### `wait`

Waits in place for a number of seconds.

```toml
[[behavior.sequences.go_to_work.steps]]
action = "wait"
seconds = 2.0
```

---

## Running Sequences

Use these Eldrin commands from `event(...)`:

### `run_sequence`

Starts the named sequence from step `0`.

```eldrin
run_sequence("go_to_work");
```

### `pause_sequence`

Pauses the currently active sequence.

```eldrin
pause_sequence();
```

### `resume_sequence`

Resumes the previously paused sequence.

```eldrin
resume_sequence();
```

### `cancel_sequence`

Stops the active sequence and clears any paused sequence.

```eldrin
cancel_sequence();
```

---

## Time-Based NPC Routines

The `time` event is the current trigger mechanism for schedules.

Example:

```eldrin
fn event(event, value) {
    if event == "time" {
        if value == 8 {
            run_sequence("go_to_work");
        }
        if value == 18 {
            run_sequence("go_home");
        }
    }
}
```

This is the recommended pattern right now:

- use `time` to decide **when**
- use sequences to describe **what**

---

## Working With Talk And Combat

Events stay in control.

Some NPCs may ignore a `talk` event and keep walking. Others may pause their sequence.

Example:

```eldrin
fn event(event, value) {
    if event == "talk" {
        pause_sequence();
        message(id(), "Hello.", "system");
    }

    if event == "goodbye" {
        resume_sequence();
    }

    if event == "attack" {
        cancel_sequence();
    }
}
```

This is intentionally explicit. The engine does not automatically decide whether an event should pause the sequence.

For shop-style `offer_inventory()` interactions, the runtime now also sends `goodbye` automatically when the buyer moves too far away or the seller's `timeout` window expires.

---

## Timeout Convention

For temporary NPC interactions, use a generic `timeout` attribute in the character **Attributes** editor.

```toml
[attributes]
timeout = 10
```

This is a simple per-NPC convention you can use in your scripts when deciding when to resume a paused sequence.

For example:

- pause while talking
- if the player walks away or the interaction ends, resume
- use the same `timeout` value for shopkeepers, dialogue, or other short interruptions

Right now, `timeout` is authoring data for your scripts. It is not an automatic scheduler by itself.

---

## Practical Advice

- Keep sequences explicit. For doors, define sectors on both sides and add `use` steps.
- Prefer named sectors/items with stable names.
- Treat sequences as authored workflows, not as full AI planning.
- Keep reactive logic in `event(...)`.

For the command details, see [Server Commands](server_commands). For the event list, see [Events](events).
