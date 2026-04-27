---
title: "Introduction"
sidebar_position: 1
---

**Eldiron Creator** is where everything comes together—a **graphical editor** that lets you build your own adventures.

![Eldiron Creator](/img/docs/screenshot.png)

On the **left side** of the screen, you’ll find a **list of tools**. These tools are used to **edit the geometry** of the currently selected region or content. The 2D and 3D geometry is displayed in the **geometry editor** in the middle of the screen.

The geometry editor can be used in 2D, orbit, isometric, and first-person camera modes. In **FirstP** mode, `Space` toggles fly navigation: move the pointer away from the center of the view to look around, use `WASD` to move, and press `Space` or `Escape` to return to normal editing. See [Camera Actions](actions#camera-actions) for the full control summary.

To the right of the screen you see the **project tree** which lists all editable items of your project, like **regions**, **characters** and more. Below the **project tree** are the settings of the currently selected **action** and beneath that the minimap of the current content. The minimap automatically adjusts to the current context (regions, tiles).

At the bottom of the screen you see the currently active **dock**, in this screenshot the tile picker. The tile picker is now both a source browser and an entry point into Eldiron’s tile authoring workflows:

- single tiles can be edited in the integrated pixel editor
- node groups can be edited in the tile node graph editor

To the right of the **dock** area is the **action list**, it lists all [actions](actions/) which can be performed at the current moment (depending on the selected geometry, item and camera).
