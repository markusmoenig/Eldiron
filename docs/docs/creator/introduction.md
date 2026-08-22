---
title: "Introduction"
sidebar_position: 1
---

**Eldiron Creator** is where everything comes together—a **graphical editor** that lets you build your own adventures.

![Eldiron Creator](/img/docs/screenshot.png)

On the **left side** of the screen, you’ll find a **list of tools**. These tools are used to **edit the geometry** of the currently selected region or content. The 2D and 3D geometry is displayed in the **geometry editor** in the middle of the screen.

The geometry editor can be used in 2D, orbit, isometric, and first-person camera modes. In **FirstP** mode, hold the right mouse button and use `WASD` to fly, then release the right mouse button or press `Escape` to return to normal editing. `Space` is only a touchpad fallback for the older pointer-from-center fly mode. See [First-Person Camera](actions#first-person-camera) for the full control summary.

The right sidebar has compact tabs for the main working contexts:

- **Project** contains the project tree, which lists editable content such as **regions**, **characters**, tilesets, and more.
- **Actions** contains the actions currently available for the selected geometry, project item, tool, and camera. Actions are grouped with labels such as **Camera:**, **Face:**, and **Bake:** and use a theme-defined color for each group. Selecting an action also displays its parameters in this sidebar.
- **Console** provides concise live-game inspection plus action, tool, and Eldrin automation commands. See [Console](console).
- **Help** provides a wrapped, read-only response area above a question field. This page is reserved for the upcoming ruleset-aware interactive help system.

The minimap remains visible below the active sidebar page and automatically adjusts to the current context, such as regions or tiles. Use `Tab` to move to the next sidebar page and `Shift+Tab` to move to the previous one when a text field is not being edited. These shortcuts also work from the Console and Help command fields so you can leave either page without reaching for the mouse.

Frequently used camera actions are also available as right-aligned icon shortcuts beside the project tabs, so changing the editor camera does not require opening the Actions page.

At the bottom of the screen you see the currently active **dock**, in this screenshot the tile picker. The tile picker is now both a source browser and an entry point into Eldiron’s tile authoring workflows:

- single tiles can be edited in the integrated pixel editor
- node groups can be edited in the tile node graph editor

The lower dock is reserved for the active content editor or picker. See [Actions](actions/) for details about the contextual action list in the right sidebar.

Creator can also be controlled by local automation through [Scepter: Remote Editing](scepter_remote_editing). Scepter is the command layer used by AI assistants, scripts, and future tools to inspect projects, preview maps, paint regions, and edit character or item data while Creator keeps ownership of undo, validation, and project state.
