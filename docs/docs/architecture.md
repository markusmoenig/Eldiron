---
title: "Eldiron Architecture"
sidebar_position: 3
---

This page gives a high-level view of how Eldiron fits together.

Many other pages document individual systems in detail. This page is the mental model first: what belongs where, how the main systems connect, and why the same game can be presented in 2D, 3D, or text.

## One World

At the center of Eldiron is a single shared world model:

- regions
- sectors
- linedefs
- entities
- items
- screens

This same authored data can be used by different presentation layers.

Examples:

- a sector is a walkable area in 2D and 3D
- the same sector can also be a room in a text-style client
- a linedef can be geometry, a connection, or both

So Eldiron is not built around separate game modes. It is built around one authored world that can be shown in different ways.

## Interaction

Eldiron separates raw input from gameplay meaning.

The basic flow is:

1. the player performs an action
2. the game resolves that into an intent
3. the world reacts

Examples:

- keyboard movement in 2D
- mouse clicks in 3D
- button widgets on screens
- terminal commands like `attack orc`

All of these can feed into the same action and intent system.

That is why the same interaction model can work across:

- 2D
- 3D
- terminal / text-style clients

See [Player Input](./characters_items/player_input) for the input-side details.

## Scripts, Rules, and Data

Eldiron intentionally splits gameplay logic into several layers.

### Scripts

Scripts control behavior and decisions.

Examples:

- reacting to events
- choosing when an NPC attacks
- opening a door
- picking a target

Scripts are the place for behavior flow.

### Rules

Rules control shared gameplay math and systemic behavior.

Examples:

- outgoing and incoming damage formulas
- progression formulas
- combat messages
- combat audio

Rules are the place for formulas and reusable game-wide logic.

See [Rules](./rules).

### Authored Metadata

Authoring metadata controls descriptive and narrative presentation.

Examples:

- room titles
- room descriptions
- connection descriptions
- per-sector presentation flags

This is edited in the **Authoring** dock and configured globally through **Game / Authoring**.

See:

- [Authoring](./creator/authoring)
- [Authoring Configuration](./configuration/authoring)

### Localization

Localization is used for shared runtime text that should exist across languages.

Examples:

- built-in `system.*` strings
- combat message templates
- shared UI text

See [Localization](./localization).

## Presentation Layers

The same game can be presented in different ways.

### 2D and 3D

The graphical clients use the authored map, tiles, avatars, widgets, and camera modes to render the world visually.

### Text / Terminal

The text client uses the same world data, but presents it as:

- room titles
- room descriptions
- exits
- visible entities and items
- messages and narration

This is not a separate gameplay system. It is another frontend onto the same world.

## Combat and Progression

The current intended split is:

- scripts decide when to attack
- `attack()` or `deal_damage()` starts combat resolution
- rules calculate outgoing and incoming damage
- messages and audio are produced from the resolved result
- progression handles XP and levels

So:

- scripts decide
- rules calculate
- presentation reports

That keeps behavior flexible without repeating all formulas in every character script.

## Creator vs Runtime

The **Creator** is the authoring environment.

This is where you:

- build maps
- place entities and items
- write scripts
- define rules
- edit locales
- edit audio FX
- enter authoring metadata

At runtime, the server and clients consume that authored project data.

Different clients can then present the same project differently:

- Creator play mode
- graphical clients
- terminal client

## Recommended Mental Model

If you are not sure where something belongs, this is the short version:

- world structure: regions, sectors, linedefs, entities, items
- behavior: scripts
- formulas and shared systems: rules
- player-facing descriptive world text: authoring metadata
- shared translated runtime text: localization
- how the game is shown: presentation layer / client

That is the bigger picture behind Eldiron’s current architecture.
