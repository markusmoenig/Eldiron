---
title: "Console"
sidebar_position: 4
---

The **Console** page in Creator's right sidebar combines quick runtime inspection with direct editor automation. It deliberately keeps common game-information commands short while using the same stable action and tool registries as the Actions sidebar and Scepter.

## Structured Feedback

Console results are structured feedback documents, not syntax-highlighted plain-text transcripts. Producers emit semantic blocks such as headings, notices, command lists, key/value lists, object lists, and code. The view maps semantic roles such as command, stable ID, key, value, success, warning, and error through a replaceable color palette.

This keeps presentation responsive in the narrow sidebar: commands and their descriptions stack instead of relying on space-aligned columns. Underlined commands and object names are interactive. Selecting one fills the Console input so it can be reviewed or completed before execution.

Plain text remains available as a copy/paste and logging fallback. Eldrin and TOML source is shown as code only when it is actually source; ordinary feedback does not depend on a regex syntax definition.

## Game Information

Game-information commands inspect the live server state for the current editor region:

```text
list
list characters
list items
focus "Old Smuggler"
focus 42
show
get health
pwd
up
```

`list` shows characters and top-level items in the current scope. `focus` accepts an exact name or runtime ID and can also find nested inventory, equipped, and container items. After focusing an object, `show` prints its details and `get` reads one attribute. Use `up` to return to the region root.

These commands require the game server to be running. They are console operations, not ruleset documentation or ruleset evaluation; ruleset guidance belongs to the separate Help system.

## Actions And Tools

The Console can discover and use Creator's registered actions and tools:

```text
actions
actions face
actions all
action camera.isometric
action face.extrude amount = 2

tools
tools all
tool geometry
tool tool.iso_paint
```

`actions` and `tools` list only entries available in the current context. Add `all` to include unavailable entries. Action IDs such as `face.extrude` and tool IDs such as `tool.geometry` are stable and non-localized; visible sidebar labels are not command identifiers.

Actions run through their normal applicability, TOML parameter, undo, dirty-state, and project notification paths. Tools use their normal deactivate/activate lifecycle. The Console does not bypass either system.

## Eldrin Automation

Prefix a one-line Eldrin automation sequence with `eldrin` when a task needs several ordered operations:

```text
eldrin console_list("characters"); editor_tool("tool.geometry"); editor_action("camera.isometric", "");
```

The Console's Eldrin host provides:

```eldrin
console_list("all");
console_list("characters");
console_list("items");
console_focus("Old Smuggler");
console_show();
console_get("health");
console_pwd();
console_up();
editor_tool("tool.geometry");
editor_action("face.extrude", "amount = 2");
```

Every call emits the same typed request used by its concise command counterpart. Requests execute in source order and stop at the first error. The Console host is trusted Creator functionality; these editor functions are not added to ordinary world, region, character, or item gameplay scripts.
