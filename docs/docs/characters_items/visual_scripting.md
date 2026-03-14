---
title: "Visual Scripting"
sidebar_position: 3
---

![Characters and Items](/img/docs/characters_items.png)

The **visual script** editor allows you to create **Eldrin** scripts without having to type code directly. It works by dragging and dropping **commands** from the left toolbar into the editor.

The editor has one folder for every **event** you support for this entity.

## Events

Dragging and dropping the **Event** item creates a new event, re-name it to the [event](events) you want to support.

As your player character probably supports several **intent** types, you can name an event to support one specific intent type, for example, naming an event **"intent: use"** would create an event folder which just handles use intents.

## Live Debugging

When you start the game from the creator, the visual script editor can show live runtime feedback directly inside the graph.

- Executed **event headers** are highlighted for the current tick.
- Executed **lines** are highlighted for the current tick.
- Executed **cells** are outlined for the current tick.
- The last **result** or **error** of a cell stays visible until that same cell runs again.
- Hovering cells in the graph shows the same status/help text as the command list on the left.

This makes it possible to see both short-lived execution flow and the last known state of important commands.

## Values And Conditions

Visual scripting now mirrors more runtime values back into the graph:

- Function calls with return values can show their current result directly on the cell.
- Assignment rows mirror the evaluated result back to the **variable** cell, so a line like `damage = random(1, 3)` shows the sampled value after execution.
- `if` conditions show `True` or `False` on the condition row.
- A false `if` is shown as a muted "not taken" state instead of an error.

This is especially useful for debugging combat, AI decisions, and temporary variables.

## Copy Existing Cells

You can still drag commands from the left command list into the editor, but you can also now drag existing cells inside the graph to copy them.

- Dragging a normal cell copies that cell.
- Dragging a function cell copies the function together with its dependency subtree.
- The copied cells receive new internal ids.
- Drop validation uses the same placement rules as drag-and-drop from the left command list.

While dragging, the editor shows whether the current target is valid before you drop.
