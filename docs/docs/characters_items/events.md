---
title: "Events and Values"
sidebar_position: 3
---

Choose an event on an Event node to start a branch. These are the named fields exposed by the event picker; read them in Filter using `event.field`, or insert them into Say/Message with `{event.field}`.

| Event | Fields | Purpose |
| --- | --- | --- |
| Startup | None | Initial setup after the owner is created. Template runs before instance. |
| Respawn | None | React to the engine's respawn event. |
| Bumped By Entity | `entity` (entity) | React to an entity colliding with this object. |
| Bumped Into Entity | `entity` (entity) | React to this character colliding with another entity. |
| Bumped Into Item | `item` (text) | React to this character colliding with an item. |
| Active | None | React to activation; Set Emit Light can follow the object's active state. |
| Time | `hour` (number) | React to a time event. For schedules, use Routine and Time Range. |
| Entered | `area` (text) | React to entering a named area. |
| Damaged | `attacker` (entity), `amount` (number), `kind` (text) | React to received damage. |
| Intent | `intent` (text), `subject` (entity), `distance`, `count` (numbers) | Handle an interaction intent. |
| Death | None | Handle death, for example dropping inventory or restoring life state. |
| Kill | `killed` (entity), `name` (text) | React to a defeated character. |

A field marked as awaiting an event has no observed value yet. Do not assume another event's fields are available.

## Named events

Use **On Event** to listen for a named event. **Notify In** schedules a named event after a delay in game minutes. This is useful for deferred checks without keeping a movement action open. Named engine events can also be listened to this way, including `entered_tile` and `left_tile` for gameplay-tagged tiles. These events currently have no named payload fields in the node adapter.

## Area transitions and navigation

**On Enter Area** compares the character's entered area with a name and routes Match or No Match. **On Area**, on the place's own graph, exposes Player Entered, NPC Entered, Player Left and NPC Left.

Entering an area and completing navigation are different conditions. Connect to **Go To → Done** when subsequent actions should wait for arrival.

## Interaction versus ruleset action

An Intent branch can filter `event.intent` against `use` to start a conversation. A UI command such as `rules.basic_attack` requests the ruleset action directly; do not add an attack intent chain to make that command work.
