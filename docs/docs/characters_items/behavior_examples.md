---
title: "Behavior Examples"
sidebar_position: 5
---

These examples use the node labels and outputs shown in Creator. Names such as Garden and Office must match places in your project.

## Player setup

Connect **Event (Startup) → Player Camera (2D Grid)** on the Player template. Character selection and player creation remain part of the game's startup flow. Add placement-specific setup to the instance's own Startup branch; both startups run, template first.

## Scheduled wandering

Connect **Routine → Time Range (14:00–16:00)**. From Inside, connect **Go To (Office) → Done → Random Walk (Office)**. Give Outside a separate destination or behavior.

Time Range reevaluates routine schedules at boundaries. Random Walk's area setting restricts where it starts, so Go To gets the character into the Office first.

## Light the North Gate bonfire

Open Elin the Gate Guard’s **Behavior Nodes** in
[Wayfarer’s Rest](../creator/wayfarers_rest). Her overnight **Time Range**
runs from **18:22** to **05:48** on the village clock. In the evening she goes
to **Gate Bonfire Post** and uses **Perform Intent** with `light` on the
placed **North Gate Bonfire**. The morning branch uses `extinguish`. The
item’s own Intent graph handles those verbs and sets its active state.

For your own routine, connect **Go To → Done → Perform Intent** so the
character reaches the item before trying the interaction. Type the intent name
and select the specific placement. To inspect the schedule in the map, select
the [Entity Tool](../creator/tools/entity) and click Elin or the bonfire.

## Craft and sell a batch

In the same village, Mara the Leatherworker’s daytime routine uses
**Use Action** for `craft_tan_hide` at the tanning rack and
`craft_make_leather_boots` at the workbench. The host checks the recipe’s
ingredients, tool and station each time. Chain the next step from **Done**
and handle **Failed** when the craft cannot run.

On her **Intent** branch, `talk` leads to **Offer Inventory** filtered to
Leather. `use` leads to **Teach** for `learn_advanced_leatherworking`.
The same character can craft, sell stock and teach through separate branches.
See [the crafting journal setup](../creator/authoring#expose-the-crafting-journal)
to expose those requirements to players.

## A guard who returns to work

Connect Routine to Lookout, choosing a relationship profile from your ruleset.

- Watching → Go To (Garden) → Done → Random Walk (Garden).
- Target Found → Engage Target.
- Engage Target's Defeated, Target Lost and Cannot Engage → Resume Routine.

Adjust reaction and escape distances on Lookout for the placement. Relationships and attack policy come from the supplied ruleset. Set movement speed on the character.

## Welcome the player to an area

On the area's graph, connect **On Area → Player Entered → Message**. NPC entry has a separate output.

Alternatively, on the Player graph, connect **Event (Entered) → Filter** with Field `event.area`, Compare Equals and Against `Garden`. Connect Match to Message with text `Welcome to {event.area}`.

## A conversation

On an NPC graph, connect **Event (Intent) → Filter** with Field `event.intent` and Against `use`. Connect Match to a greeting Prompt. Add an answer row for each reply; its labeled output can connect to another Prompt, an action, or nothing to end the conversation. A Back answer can connect to the greeting Prompt.

For a quest offer, connect a **Quest Guard** set to `bell_below / Not Started` to the offer answer's **Show if** input. Connect that answer's output to Set Quest (`bell_below / Active`), then Message or Add Item. A later answer can use a Quest Guard set to Active and connect to Set Quest (Completed). In Hideout 2D, Mara's reward answers combine a Quest Guard, Player Attribute Guard and Item Guard. Each player's state is independent. Connect an action's output to a Prompt if the conversation should continue. End the interaction with Resume Routine when the character should return to work.

## Death and return

A character death branch can use **Event (Death) → Drop Items → State (Alive) → Teleport (Start)**. Choose the destination region if necessary. Decide separately whether to restore attributes such as health and whether inventory should be dropped. Use the dedicated State node for lifecycle changes; hiding a character is not the same as killing it.

## Player attacks

Bind an input or screen button to `rules.basic_attack`. The ruleset handles the action. No Player attack node chain is required. Use Player nodes for reactions such as Kill → Message, or Entered → Teleport.
