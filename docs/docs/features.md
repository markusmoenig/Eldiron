---
title: "Features"
sidebar_position: 2
---

Eldiron is not just a map editor or scripting tool. It is a game creation framework built around one shared world model and several connected authoring systems.

This page highlights the main strengths of the current Eldiron workflow.

## One World, Multiple Presentations

The same game data can be presented in different ways:

- 2D
- 3D
- terminal / text-style clients

That means sectors, linedefs, entities, items, scripts, and rules do not belong to just one visual mode.

A game can be explored graphically, or through text, using the same authored project data.

## Playable Before Final Art

One of Eldiron’s strongest advantages is that a game can already be:

- prototyped
- balanced
- tested
- and played

before the final visual assets exist.

Because of the text-style client and authoring metadata, you can validate:

- movement flow
- exits and room structure
- combat and progression
- messages and pacing
- item and NPC interactions

without waiting for finished tiles, avatars, or UI art.

## Unified Interaction Model

Eldiron separates raw input from gameplay meaning through **actions** and **intents**.

This allows the same interaction model to work across:

- keyboard controls
- mouse clicks
- UI widgets
- terminal commands

So `attack`, `use`, `take`, movement, and other interactions do not need separate gameplay systems for each client type.

## Scripts and Visual Scripting

Behavior can be authored in two ways:

- **Visual Scripting**
- **Eldrin Script**

Visual scripts are translated into Eldrin Script, so both paths are part of the same runtime model.

This makes Eldiron useful both for:

- people who prefer node-based authoring
- people who want direct script control

## Shared Gameplay Systems

Eldiron includes game-wide systems that reduce repetition across characters and items.

These include:

- **Rules** for combat and progression formulas
- **Localization** for shared runtime text
- **Audio FX** for generated sound effects
- **Authoring** for descriptive world metadata

Instead of hardcoding everything into per-character scripts, you can centralize shared logic and presentation.

## 2D and 3D Editing

The Creator supports both 2D and 3D editing workflows.

You can:

- build regions in 2D
- inspect and edit them in 3D
- place entities and items
- test gameplay directly from the editor

This makes it possible to work visually while still keeping the project data flexible enough for non-visual clients too.

## Procedural 3D Building Tools

Eldiron also includes non-destructive 3D building tools that let you create structures and details directly from map geometry.

Examples include:

- houses and room shells from geometry profiles
- palisades along linedefs
- fences along linedefs
- roofs on selected structures
- stairs on sectors
- campfires and other scene details

This is useful because many 3D structures can be authored from the same map data you already use for gameplay and navigation, instead of being modeled as separate static assets first.

So Eldiron’s 3D workflow is not only about placing tiles in space. It also supports procedural scene building for things like:

- houses
- palisades
- fences
- settlement details

That makes it much faster to block out and iterate on playable 3D spaces.

## Reusable Content

Characters and items are created as reusable templates.

They can define:

- attributes
- scripts
- visuals
- input mappings
- authoring text

and then be instantiated into regions as needed.

This makes it easier to iterate on gameplay without manually rebuilding content in every map.

## Global Rules, Locales, and Authoring

Game-wide data is organized clearly through dedicated project nodes.

Examples:

- **Settings**
- **Authoring**
- **Rules**
- **Locales**
- **Audio FX**

This gives Eldiron a stronger structure than “everything lives in scripts,” and makes large projects easier to maintain.

## Creator Workflow

The Creator brings the main systems together in one place:

- tools for world editing
- docks for tiles, data, scripts, and authoring
- a project tree for the whole game
- in-editor play and debugging

That makes Eldiron feel less like a loose collection of tools and more like a unified creation environment.

## What This Adds Up To

Eldiron is strongest when you think of it as:

- one world model
- one interaction model
- multiple presentation layers
- shared systems for rules, text, and progression

That combination is what makes it possible to build games that are playable early, adaptable across client types, and easier to extend over time.
