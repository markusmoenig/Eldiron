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

## A guard who returns to work

Connect Routine to Lookout, choosing a relationship profile from your ruleset.

- Watching → Go To (Garden) → Done → Random Walk (Garden).
- Target Found → Engage Target.
- Engage Target's Defeated, Target Lost and Cannot Engage → Resume Routine.

Adjust reaction and escape distances on Lookout for the placement. Configure relationships and attack policy in Game / Rules. Set movement speed on the character.

## Welcome the player to an area

On the area's graph, connect **On Area → Player Entered → Message**. NPC entry has a separate output.

Alternatively, on the Player graph, connect **Event (Entered) → Filter** with Field `event.area`, Compare Equals and Against `Garden`. Connect Match to Message with text `Welcome to {event.area}`.

## A conversation

On an NPC graph, connect **Event (Intent) → Filter** with Field `event.intent` and Against `use`. Connect Match to a greeting Prompt. Add an answer row for each reply; its labeled output can connect to another Prompt, an action, or nothing to end the conversation. A Back answer can connect to the greeting Prompt.

For a quest offer, connect a **Quest Guard** set to `bell_below / Not Started` to the offer answer's **Show if** input. Connect that answer's output to Set Quest (`bell_below / Active`), then Message or Add Item. A later answer can use a Quest Guard set to Active and connect to Set Quest (Completed). Mara's branch combines a Quest Guard, Player Attribute Guard and Item Guard on her reward answers. Each player's state is independent. Connect an action's output to a Prompt if the conversation should continue. End the interaction with Resume Routine when the character should return to work.

## Death and return

A character death branch can use **Event (Death) → Drop Items → State (Alive) → Teleport (Start)**. Choose the destination region if necessary. Decide separately whether to restore attributes such as health and whether inventory should be dropped. Use the dedicated State node for lifecycle changes; hiding a character is not the same as killing it.

## Player attacks

Bind an input or screen button to `rules.basic_attack`. The ruleset handles the action. No Player attack node chain is required. Use Player nodes for reactions such as Kill → Message, or Entered → Teleport.
