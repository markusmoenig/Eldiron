---
title: "Behavior Nodes"
sidebar_position: 2
---

A behavior graph connects events to actions. Nodes carry their settings, and labelled output terminals show the possible outcomes.

## Create a branch

1. Select **Behavior Nodes** for a character, item, instance, world or region in the project tree. Named areas can also own graphs.
2. Open **Node List** in the sidebar. Available nodes depend on the selected object; colors identify their groups.
3. Drag an Event node into the graph and choose an event, such as Startup.
4. Drag an action node alongside it and connect the event's output to the action's input.
5. Set the action's parameters. Start the game and inspect the branch's feedback.

The usual flow is left to right. Output names matter: Go To's Done means arrival, while Random Walk's Started means wandering has begun.

## Events and values

Event nodes display the named values they offer and the latest received values while running. The context follows the connections, including through filters.

For an Entered event, compare `event.area` with `Garden` using Filter. In Say or Message, write `Welcome to {event.area}` to insert the current area's name. A reference must be supplied by that event; not every event has the same fields. See [Events](events).

Filter reads `event.<field>` or `attr.<name>` on the acting object. For example, compare `attr.health` with `0`. In an area graph the acting character is the entrant. A missing attribute takes No Match; a missing event field reports an error. Comparisons include Equals, Not equal, Contains, Greater than and Less than, with Match and No Match outputs.

For quest progress, use Quest State and Set Quest rather than an NPC attribute. The state belongs to the participating player; the same NPC can therefore offer different answers to different players.

Prompt answers have Show if inputs on the left and answer outputs on the right. Leave Show if unwired to offer an answer unconditionally. Connect Quest Guard, Item Guard or Player Attribute Guard to restrict it; several guards on one answer all have to pass. Their nodes and connections light up when the Prompt evaluates them, and greenish/reddish borders show the last result.

## Ownership

[Templates and instances](getting_started#templates-and-instances) run separate graphs. An area graph receives transitions for its own place; use On Area to distinguish players and NPCs entering or leaving. A character graph can instead use On Enter Area to match a destination name.

World and region graphs have a smaller node catalog. A node's presence in another object's catalog does not imply that every action is valid for every owner: character-only operations report an error if they have no character to act on.

## Editing and feedback

Use the graph's pan and zoom controls to navigate. Use the global Undo and Redo commands for graph edits. Hover over catalog entries for help.

Inline text fields commit on Return. Multiline text editors use Ctrl+Enter to save and Escape to cancel. The Branches list selects a flow; its branch tools can tidy or remove that branch.

While playing, executed nodes and connections are highlighted. Long-running activities remain highlighted; event nodes show received values, and status text explains execution or errors. Time Range uses greenish/reddish borders to show whether the current time is inside or outside its range. Condition feedback is separate from execution highlighting.

Committed settings and structural edits refresh the running graph. Invalid updates report diagnostics and retain the last valid behavior. Startup effects are not replayed just because a graph was edited; use a new game run to test startup changes. Live edits apply to the selected graph, including instance graphs, independently.

See the [Nodes dock](../creator/docks/behavior_nodes) and [Debug](../creator/debug) pages.
