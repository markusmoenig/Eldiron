---
title: Eldiron Development Update 1
authors: [markusm]
tags: [development update]
---

<!-- truncate -->

As I’m working on the next release of Eldiron, I wanted to share a quick update on what’s happening behind the scenes. I originally planned to release new versions every 2–3 weeks, but this round is taking a bit longer—I’m deep in core systems that need to be just right before moving forward.

## Render Graph

I’ve been integrating the new render graph system, which defines how the scene is drawn using a node-based approach. This allows plugging in effects like fog, ambient lighting, or even a fully procedural sky. More interestingly, this system ties directly into the map’s geometry—vertices, linedefs, and sectors.

You can now attach nodes directly to geometry to define local effects like lighting or fog, tweak terrain blending, or control glossiness and reflection on tiles. These graphs are reusable across multiple areas, making it easy to organize logic and effects in a modular way.

This isn't just visual fluff—eventually, these nodes will also drive gameplay interactions. Think of a node that defines a magical barrier, a trigger zone, or a destructible structure. The goal is to unify rendering, behavior, and logic into a single editable system.

And yes—this applies not just to 3D but also to 2D and isometric modes. The same systems work across all views, which keeps things consistent and flexible.

## Terrain System

The new terrain tool lets you sculpt elevation with custom brushes—raise, lower, smooth, or roughen areas as you see fit. Tiles and materials blend automatically using rules, so you can paint smooth transitions and beautiful terrain with minimal effort.

All of this ties into the render graph too, which means terrain isn’t just a visual element—it can be procedural, dynamic, and reactive.

The inspiration is somewhere between “retro charming” and “early WoW Vanilla”—natural, stylized, and smooth, but still performant and easy to work with.

## Asynchronous Streaming Terrain and Geometry

To support massive maps, terrain and geometry are now chunked and streamed in and out as needed. Unlike a game engine that only cares about the player’s position, Eldiron Creator has to update everything live as you edit.

That means background threads rebuild chunks, update render data, blend tiles, and apply node graphs without freezing the UI. It’s been a lot of low-level work, but it’s finally paying off—terrain edits feel smooth, and you can paint or modify geometry interactively.

## What’s Next

I’m currently working on the first pass of procedural path nodes—letting you carve winding roads and trails that modify elevation and materials based on rules. These will connect with the graph system too, opening up possibilities for bridges, stairs, or terrain-aware gameplay triggers.

—

That’s it for now! Thanks again for following along—and for your support, especially on Patreon. The next version will be out once it’s stable and fun to use.

Take care,
Markus
