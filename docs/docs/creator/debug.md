---
title: "Debug"
sidebar_position: 5
---

Creator's debugging support has two complementary views: the **Debug** page in the right sidebar shows runtime diagnostics, while the [Eldrin Script Editor](docks/eldrin_script_editor) visualizes the workflow of a running script directly in its source.

## Runtime Diagnostics

The Debug sidebar is a read-only, syntax-highlighted log for server startup messages and runtime diagnostics. It is separate from the [Console](console): Console is an interactive command and inspection system, while Debug displays messages produced by the running game and its scripts.

Diagnostic severity is conveyed by text color:

- ordinary status and debug messages use the normal text color
- warnings use yellow
- errors use red

Internal markers such as `[warning]` and `[error]` select the appropriate highlight color, but are omitted from the displayed and copied text. Long messages wrap inside the sidebar and retain their severity color across every visual line.

When new log content contains an explicit warning or error marker, Creator automatically opens the Debug page. Ordinary status messages do not interrupt the current sidebar page.

Use `Ctrl/Cmd+Shift+J` to open Debug directly. `Tab` and `Shift+Tab` move between all sidebar pages.

## Script Workflow Visualization

While the game is running, Creator collects execution information from Eldrin scripts. Open the **Eldrin Scripting** editor for a World, Region, Character, or Item to see the latest matching runtime workflow overlaid on the source:

- recently executed lines receive a translucent highlight and a colored rail
- the latest executed line is emphasized
- taken branches are distinguished in the workflow
- captured variable values can appear beside the line as compact inline badges

The visualization follows the selected script and its matching runtime object. It updates from the latest recorded invocation, making it useful for seeing which path the game actually took and which values were involved.

This is an execution visualization rather than a breakpoint debugger: it does not pause the game, single-step statements, or replace log messages. Use the source overlay to understand control flow, and the Debug sidebar to inspect warnings, errors, startup problems, and messages emitted by Eldrin's `debug()` command.

