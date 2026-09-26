---
title: "Node Reference"
sidebar_position: 4
---

The Node List is contextual. Character movement and combat nodes are offered for characters; On Area belongs to an area graph. Hover help explains each node in the editor.

## Entry and logic nodes

| Node | Settings and behavior | Outputs |
| --- | --- | --- |
| Event | Select an engine event; displays its named values. | Event flow |
| On Event | Listen for a named event. | Event flow |
| Routine | Start normal character behavior after Startup or Resume Routine. | Routine flow |
| On Enter Area | Match an entered area by exact name. | Match, No Match |
| On Area | Handle transitions for the selected place. | Player Entered, NPC Entered, Player Left, NPC Left |
| Filter | Compare `event.<field>` or `attr.<name>` against a value. | Match, No Match |
| Time Range | Start/end as HH:MM. Start included, end excluded; overnight ranges supported. Equal times mean all day. | Inside, Outside |
| Inventory Has | Check inventory for an item. | Has, Missing |
| Entities In Radius | Check for another living entity inside the owner's collision radius. | Empty, Occupied |

## Movement and combat

### Go To

Set a named destination and speed multiplier. **Done** runs after arrival; **Error** handles an invalid destination, blocked navigation or interrupted movement. Follow Done with an action that needs the character to be there.

### Random Walk

Set an optional area name, step distance, speed multiplier and maximum pause in game minutes. An empty area uses the current area. **Outside Area** runs if the character is elsewhere; this node does not take them there. **Started** means wandering began, not that it finished. A maximum pause of zero removes the intentional pause between walks.

### Lookout

Choose a ruleset relationship profile. Reaction and escape distances are integer node settings from 1 to 20, initially populated from ruleset defaults. **Watching** lets normal behavior continue while the watcher remains active. **Target Found** interrupts it and sets the shared current target.

### Engage Target

Pursue and attack the current target using a ruleset combat profile. The ruleset controls available attacks, range, costs and cooldowns. Movement uses the character's speed; Lookout supplies the escape distance. Wire **Defeated**, **Target Lost** and **Cannot Engage** to appropriate follow-up behavior, commonly Resume Routine.

### Use Action

Request a ruleset attack once using `event.subject`, or the current target when no subject is supplied. **Done** means accepted; **Failed** means refused, for example due to range or cooldown. The current picker offers attacks targeting characters. This is for graph-initiated actions; ordinary ruleset UI commands already execute their actions directly.

### Resume Routine

Restart normal behavior after a temporary interaction. Connect it after the interaction's completion, rather than after a long-running action's Started output.

## State, text and inventory

| Node | Purpose |
| --- | --- |
| Say | Display text; supports named event references such as `{event.area}`. |
| Message | Send text to the game log with a category; supports event references. |
| Set Attribute | Write a named attribute with Text, Number, Boolean or Toggle type. |
| State | Set Alive, Dead, Sleeping or Unconscious. Visibility remains a separate attribute. |
| Teleport | Move the character to a named area, optionally in another region, after the current event finishes. |
| Player Camera | Select 2D, 2D Grid, Isometric, First Person or First Person Grid after character creation. |
| Add Item | Add an item from ruleset item templates to inventory. |
| Drop Items | Drop matching inventory; leave the filter empty to drop everything. |
| Offer Inventory | Present carried items to the interaction's initiator; an empty filter offers everything. |
| Set Emit Light | Follow Active, force On, or force Off. |
| Notify In | Schedule a named event after a delay in game minutes. |

## Conversations

**Prompt** shows one spoken line and its answers. Each answer has an output labeled with its text; connect it to another Prompt or an action. Done handles dismissal, including walking away, or a line with no available answers. A Prompt offers up to six answers. Each answer also has an optional **Show if** input. Connect guard nodes there to control whether that answer appears. Multiple guards on one input must all pass.

Blank choice rows keep their output numbers. If conditions hide every answer, the text is shown and Done runs immediately. Only one conversation may be open for a speaker at a time.

**Dialog** opens a named dialogue from the project's authoring data. Its Opened output runs as soon as the dialogue opens. Use Prompt when the conversation should live in the branch itself.

**Quest State** branches on the participating player's Not Started, Active or Completed state for a Quest ID. **Set Quest** starts, completes or resets that quest on the player, with Done, Unchanged and Failed outputs. **Quest Guard**, **Item Guard**, and **Player Attribute Guard** control Prompt answer visibility through Show if inputs. A Quest Guard can also check Not Completed. On an NPC's Intent branch, the participating player is `event.subject`; on a player graph, the player is the graph owner. Quest IDs use letters, numbers, `_` and `-`; an unset quest is Not Started.

See [examples](behavior_examples) for complete branch layouts.
