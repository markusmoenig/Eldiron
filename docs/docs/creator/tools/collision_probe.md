---
title: "Collision Probe Tool"
sidebar_position: 9
draft: true
---

The **Collision Probe Tool** (keyboard shortcut **C**) previews walkability and collision behavior directly in 2D and 3D editor views.

Use it when checking whether an actor can pass through an opening, climb steps, cross a bridge, reach a balcony, or follow a scripted `goto` route. The probe uses the same mesh floor and collision logic as runtime movement, including actor radius, step height, blocking geometry, and floor support sampling.

## Creating A Probe

- **Click** once to place the start point.
- Move the mouse to preview the next segment.
- **Click** again to commit the segment and continue a multi-segment path.
- Press **Escape** to finish the current polyline.
- Press **Escape** again with no active polyline to clear the probe overlay.

In 3D views, the tool keeps preview points on the current movement plane while drawing, so the path remains stable in isometric and perspective views.

## Reading The Overlay

- **Green** means normal walkable movement.
- **Yellow** means a step up or step down.
- **Magenta** shows the scripted `goto` route preview.
- **Orange** means contact with blocking geometry while still making useful progress.
- **Red** means blocked movement or no valid floor.
- **Blue** support markers show reachable floor samples around the actor radius.
- **Gray** support markers show nearby floor samples outside the reachable step height.

The overlay uses dark outlines around the route lines so probes stay readable on bright, dark, and similarly colored geometry.

## Runtime Goto Checks

The magenta route previews whether scripted `goto` style movement can find a route to the target. For local stair and bridge movement, runtime `goto` first tries the same stair-aware floor stepping used by direct actor movement, then falls back to navgrid pathing when direct movement cannot make useful progress.

This makes the tool useful for verifying short pathing details such as small stairs, balcony lips, ramps, narrow bridges, and cutouts before testing them in play mode.
