# Parameter-rich graph editor

`theframework::thegraph` is a new editor component, separate from the legacy
`TheNodeCanvasView`. It has no gameplay runtime or dependency on Eldrin.

Run the interactive example from the repository root:

```sh
cargo run -p theframework --example behavior_graph --features ui
```

Render a PNG without opening a window or compiling a graphics backend:

```sh
cargo run -p theframework --example behavior_graph --no-default-features --features ui -- --snapshot /tmp/behavior-graph.png
```

## Prototype interactions

- Drag empty canvas to pan; wheel at the pointer to zoom between 25% and 300%.
- Drag a node body/header to move it.
- Drag number/time pills to change them. Click the left/right half of a selector
  to cycle its values. Click text fields to focus and select their content; type
  to replace it. Arrow/Home/End keys move the caret, Shift extends selection,
  Cmd/Ctrl+A selects all, Enter/Tab commits, Escape restores, and clicking away
  commits one undoable edit. Long fields scroll horizontally to keep the caret visible.
- Drag a terminal onto another terminal to connect, starting at either end.
- Click a connection and press Delete to remove it; Escape cancels a gesture.
- The toolbar time slider and area selector supply preview context.
- Trace toggles **simulated** execution, independently of condition truth.
- **+ Node** opens the searchable definition catalog. Type to filter; click or
  press Enter to add a node. Escape closes the picker.
- Click the Event field to select Arrived, Damaged, or the sample custom event.
- Click **[=]** beside a bindable parameter to choose a literal or compatible
  field from a connected Event node. Clicking a bound value also opens the picker.
- Add Choice demonstrates row-aligned ports with stable IDs. Undo/Redo also cover
  added choices, moved nodes, parameter gestures, and connection changes.

The example has no actual NPC execution. Dialogue lines use mouse-selectable
presets and choices support text editing. On Arrived -> Text Equals compares
the preview event destination `garden` with a freely editable string. The map thumbnail is sample artwork. The context and trace controls are
explicitly a demonstration, not an interpreter for the example graph.

## Interfaces

| Component | Responsibility |
| --- | --- |
| `GraphDefinitions` | Node/event schemas, instance creation, typed binding validation/resolution |
| `GraphPicker` | Reusable searchable, scrollable popup with stable item IDs |
| `GraphDocument` | Serializable authored nodes, rows, ports, and connections |
| `GraphEditor` | Viewport, input capture, selection, gesture editing |
| `GraphEdit` | Completed reversible edits; hosts store history and persist changes |
| `GraphControls` | Value interaction, formatting, optional custom painting |
| `GraphTextInput` / `GraphTextFocus` | Focused single-line editing, selection and grouped undo |
| `GraphConnectionPolicy` | Host semantic constraints beyond direction/kind checks |
| `GraphContext` | Read-only node observations and connection trace state |
| `GraphPainter` | Drawing backend and preview rendering |
| `GraphAssets` | Host-owned preview lookup for the provided raster backend |

The host owns the document. Gestures update it for immediate feedback and emit
one edit on completion; cancellation restores the original value. Drain
`take_edits()` into the application's undo system. Cancel active gestures before
replacing documents or performing external edits.

Each input position is in logical coordinates local to the graph panel. All
layout, connections and hit testing share `GraphViewport`. The raster backend
accepts device density separately and rasterizes curves/text at that density
and zoom; preview assets remain bitmaps. A panel host can render into a local
framebuffer and composite it, or provide another `GraphPainter`. The example
uses the framework application callbacks directly, without embedding ordinary
text widgets or adding a cast to the legacy widget hierarchy.

Ports have direction independently of side. Left, right, top, and bottom are
supported. A left/right port can reference a row ID for alignment; otherwise
`position` is the normalized position along the edge. Row height and node width
are authored in graph units. Keep node widths greater than 36 units. Connection
IDs and port IDs do not change when rows/nodes are reordered.

`GraphControlValue::Custom` carries a versionable host payload. Override
`GraphControls` to implement its interaction/formatting/custom drawing. The demo
uses this for time-of-day pills. Read-only labels and asset thumbnails are also
node-native rows.

Context is inspected only for visible nodes, and cannot be used as an execution
callback. The host supplies a consistent live, preview, or recorded snapshot.
`Unknown` is the default condition. Green/red borders show context truth;
execution/selection have separate outer rings. Status text remains available; a true condition never implies execution. Actual execution
tracing, invocation selection, and snapshot capture belong to the future host.

## Next integration steps

The prototype leaves full application menus, multi-selection, an event-schema
editing UI, save/load UI, and gameplay execution to later work. The document is serializable; production loading will need schema
migration and validation of imported documents. The connection policy can add
cycle/cardinality rules appropriate to the eventual graph engine.

Use the example to settle the look and mouse interaction before adding the
Creator panel adapter and the dedicated behavior runtime.

## Rendering and text input

The raster backend copies opaque scanline interiors for rounded controls and
background/grid fills, blending boundary coverage separately. Curves still use
the general path rasterizer. The example targets 60 updates per second; actual
frame rate depends on rendering cost and display density. For a reproducible
warm-render timing, append `--density 2 --benchmark` to the snapshot command.

Hosts feed `GraphTextInput` to the focused field before processing canvas
shortcuts. Text editing uses grapheme boundaries and supports insertion of whole
strings (including host-provided paste), forward deletion and backspace. The
current framework window adapter maps both native Backspace and Delete to its
single `TheKeyCode::Delete`; this example treats that as backspace. Input-method
composition and pointer-based caret placement are not implemented yet. Clicking
a field selects its content; keyboard navigation places the caret precisely.

Call `finish_text(document, true)` on host focus loss or before host-level undo.
Call it with `false` to cancel. Focus is editor state and is never serialized.

## Definitions and bindings

The host-owned catalog lives in `examples/behavior_graph/catalog.rs`. Every
initial node and every node added by the picker is instantiated from that
catalog. The generic Event definition stores a selected event ID; its displayed
`On Arrived` title is derived from the event label. There is no Arrived-specific
node type. The initial example flow is Event -> Text Equals -> Say; the Dialogue
sample is below it and can be reached by panning.

`GraphNodeDefinition` describes parameter defaults/types, presentation, ports,
and an optional event-selector parameter. `instantiate` creates fresh node,
row, and port instance IDs while preserving definition-local keys. Definition
IDs and parameter/port keys are author-assigned stable identifiers: keep them
when renaming or reordering a published schema. `from_template` is a convenience
for initial prototypes; assign semantic keys before publishing a definition.

`GraphEventDefinition` uses stable event and field IDs with typed payload fields.
Built-in and custom events register through the same method. The sample custom
Delivery Arrived event is registered in the catalog; an editor for authoring new
event schemas is not part of this milestone.

A row retains its literal value and optionally references an explicit source
Event node, event ID, and field ID. Switching back to Literal restores the saved
literal. Only compatible fields from reachable event entries appear in the
binding picker. Changing the event preserves existing references: invalid
references produce a failure indicator/status until explicitly rebound. A
missing preview sample yields Unknown, rather than a false condition or a reused
value from another event. Bindings and selected events round-trip in the graph
document; schema definitions are separately serializable host assets.

The demo explicitly supplies sample payloads for each event. Resolving these
samples is read-only inspection, not actual event delivery or node execution.

## Creator host

Creator embeds this renderer in its Nodes dock (`creator/src/docks/nodes`).
The behavior catalog is host-owned; the framework does not depend on gameplay
node names or the project model. Future screen/attribute catalogs can use these
same interfaces. Creator uses named event fields and Filter / Say authoring,
while this standalone example retains explicit bindings as an API demonstration.
Neither host currently executes a gameplay graph.
