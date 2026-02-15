---
title: "Project Tree"
sidebar_position: 2
---

The project tree, located to the right of the *Eldiron Creator* contains all editable content of your game. Use the **+** and **-** buttons at the bottom of the tree to add or remove content, to the right of these buttons the current content context is displayed as well as **import** and **export** buttons.

Selecting specific content in the project tree will display a corresponding editor dock widget.

---

## Regions

Regions are the maps in your game which define the world, dungeons and towns.

You use the [geometry tools](tools/overview#map-tool-specifics) to create geometry for the regions. Regions can be viewed using a **2D** or various **3D** cameras.

## Characters

A **character template** is a reusable blueprint that defines the **behavior, attributes, and appearance** of a character in the game.

You can edit character behavior using either **Visual Scripting** or **Eldrin Scripting** or edit the initial **Attributes** of the character.

You can instantiate a character template into the map of the region by simply dragging and dropping the character template into the map (Click left of the *Name* item and drag).

You can use the [Entity Tool](tools/entity) to move or delete character instances after creation.

## Items

Item templates have similar functionality as *characters templates* but define a static or dynamic item in the game world.

Like with *characters* you can edit item behavior using either **Visual Scripting** or **Eldrin Scripting** or edit the **Attributes** of the item.

You can instantiate an item template into the map of the region by simply dragging and dropping the item template into the map (Click left of the *Name* item and drag).

You can use the [Entity Tool](tools/entity) to move or delete item instances after creation.

## Tilesets

![Tileset](/img/screenshots/tilesets.png)

Tilesets are PNG images with grid based tiles of pixel art. After importing a tile set and applying the correct **Grid Size** you can select individual or multiple tiles (click and drag horizontally or vertically to select) and add them to the tile picker.

Before adding the tile(s) make sure you selected the role as needed (to be able to filter and sort tiles correctly) and you can select one of three import modes:

- **Single**. Add the selected tiles as individual tiles.
- **Anim**. Add the selected tiles as one animated tile.
- **Multi**. Add the selected tiles as one big tile, containing multiple tiles.

In the tile picker you can further edit tiles, both their meta data and their pixels, using the corresponding *Actions*. 

Added tiles are shown with a slight gray overlay in the tileset editor, making it easy to see which files you already added. You can delete a tileset any time, already added tiles will not be affected.

## Screens

Screens in *Eldiron* define the visible user interface of your game. Paint background decoration with the *Rect* tool and create **Sectors** and apply tiles.

See the **Working with Screens** chapter (coming soon).

## Assets

Assets you added for your game will be listed here. Currently supported assets:

- **TTF** fonts for in-game text drawing.

## Palette

Shows the colors of your palette. Use the palette based **Actions** to clear or import palettes.

## Game

In the game section you can select:

- **Settings**. Edit your game settings, see all supported settings in the [reference](../reference/configuration).
- **Debug Log**. Displays state during game play, important especially to diagnose server startup or runtime errors. Shown by default after starting the game server.
