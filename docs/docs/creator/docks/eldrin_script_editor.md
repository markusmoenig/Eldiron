---
title: "Eldrin Script Editor"
sidebar_position: 2
---

![Eldrin Script Editor](script_editor.png)

The Eldrin script editor is active when an *Eldrin Scripting* item is selected for a world, region, character, or item.

It allows you to directly edit in the [Eldrin Scripting Language](/docs/characters_items/eldrin_scripting_language).

While the game is running, the editor also visualizes the latest execution workflow for the selected World, Region, Character, or Item script. Executed lines and taken branches are highlighted directly in the source, and captured values can appear as inline badges. This live overlay complements the runtime messages in the sidebar's [Debug](/docs/creator/debug) page.

This editor writes gameplay scripts for the selected project object. Creator automation functions such as `editor_action` and `editor_tool` belong to Scepter's separate editor-automation host; typing them into an ordinary world, region, character, or item script does not make them available there. See [Scepter: Remote Editing](/docs/creator/scepter_remote_editing#creator-actions-and-tools).
