---
title: First Terrain Editing Screenshots
authors: [markusm]
tags: [development update]
---

<!-- truncate -->

As I work on the next release of Eldiron, I’ll post progress screenshots along the way. Here are two shots of the terrain modeling tools in action.

![Terrain Editing](/img/terrainedit.png)

In the first screenshot, you can see how procedural nodes can be applied to sectors and linedefs to deform and colorize the terrain. For example, we flatten the ground and apply a stone texture, while the linedef creates a smooth path into the mountains.

![Terrain Editing](/img/terrainbrushes.png)

The second screenshot shows the new terrain tool, where we can sculpt the terrain using brushes. Rendering with terrain modifiers happens asynchronously in a background thread, providing a smooth and responsive editing experience.

—

Take care,
Markus
