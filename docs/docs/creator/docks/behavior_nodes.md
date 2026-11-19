---
title: "Nodes"
sidebar_position: 2
---

The Nodes dock edits configuration and behavior for the object selected in the project tree. Open **Entity Nodes** for initial settings, or **Behavior Nodes** for event-driven actions. Drag nodes from the sidebar's **Node List** onto the canvas.

Nodes keep their parameters beside the flow they control. Connect output terminals to inputs and follow their labels to see when the next node runs. The Branches list lets you focus on a particular entry flow. Node groups are identified by color; hover catalog entries for help.

Use pan and zoom to navigate, and global Undo/Redo for edits. Inline text commits on Return. Expanded multiline text saves with Ctrl+Enter and cancels with Escape.

While the game runs, node and connection highlights show execution, event rows show received values, and compact status text reports results. Time Range borders show the current condition independently of execution. Template and instance graphs have separate feedback and live updates.

Start with [Behavior Nodes](../../characters_items/behavior_nodes), the [node reference](../../characters_items/node_reference), and [examples](../../characters_items/behavior_examples). Errors appear in [Debug](../debug).

See [Entity Nodes](../../characters_items/entity_nodes) for identity, appearance, input bindings, and template/instance configuration.

Behavior nodes use the project's supplied gameplay rules. The generic ruleset
editor is internal maintenance tooling and is hidden in standard Creator builds.

## Teach

**Teach** is available on character behavior graphs. Choose a supplied learning
action such as `learn_boneworking`. The NPC owning the graph is the teacher;
`event.subject` identifies the learner from the interaction or dialogue.

The node invokes the shared authoritative learning action. The NPC must offer
the lesson through its profession or explicit `teaching` attribute, and range,
skill, costs and ownership checks still apply. **Done** means the lesson was
learned or was already permanently known; **Failed** means it could not be
performed. A repeated learned lesson charges nothing. Equipment guidance does
not count as permanent learning.

Use the supplied manual with its study action for book learning. The Teach
picker offers teacher actions, excluding book study and ordinary combat actions.
