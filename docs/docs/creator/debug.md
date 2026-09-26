---
title: "Debug"
sidebar_position: 5
---

Creator's debugging support has two complementary views: the **Debug** page in the right sidebar shows runtime diagnostics, while the [Nodes dock](docks/behavior_nodes) visualizes executed nodes and connections.

## Runtime Diagnostics

The Debug sidebar is a read-only, syntax-highlighted log for server startup messages and runtime diagnostics. It is separate from the [Console](console): Console is an interactive command and inspection system, while Debug displays messages produced by the running game and its behaviors.

Diagnostic severity is conveyed by text color:

- ordinary status and debug messages use the normal text color
- warnings use yellow
- errors use red

Internal markers such as `[warning]` and `[error]` select the appropriate highlight color, but are omitted from the displayed and copied text. Long messages wrap inside the sidebar and retain their severity color across every visual line.

When new log content contains an explicit warning or error marker, Creator automatically opens the Debug page. Ordinary status messages do not interrupt the current sidebar page.

Use `Ctrl/Cmd+Shift+J` to open Debug directly. `Tab` and `Shift+Tab` move between all sidebar pages.

## Node execution feedback

Open the selected object's **Behavior Nodes** graph while the game runs. Executed nodes and connections are highlighted, long-running activities stay visible, and Event nodes show the latest payload. Status text reports outcomes and errors. Time Range borders show its current condition independently of execution.

Template and instance graphs have separate feedback. Use [Nodes](docks/behavior_nodes) to follow a branch and the Debug sidebar to inspect runtime errors. This feedback does not pause or single-step the game.
