use crate::prelude::*;

/// All events which are handled by the framework
#[derive(Clone, Debug)]
pub enum TheEvent {
    // These events are passed to the on_event function of the widgets and cover user interaction.
    Context(Vec2<i32>),
    MouseDown(Vec2<i32>),
    Hover(Vec2<i32>),
    MouseDragged(Vec2<i32>),
    MouseUp(Vec2<i32>),
    MouseWheel(Vec2<i32>),

    KeyDown(TheValue),
    KeyUp(TheValue),
    KeyCodeDown(TheValue),
    KeyCodeUp(TheValue),
    ModifierChanged(bool, bool, bool, bool),
    DropPreview(Vec2<i32>, TheDrop),
    Drop(Vec2<i32>, TheDrop),
    TileDropped(TheId, Uuid, usize),

    // These events define widget states.
    StateChanged(TheId, TheWidgetState),
    SetState(String, TheWidgetState),
    SetStateId(Uuid, TheWidgetState),

    DragStarted(TheId, String, Vec2<i32>),
    DragStartedWithNoImage(TheDrop),

    ValueChanged(TheId, TheValue),
    SetValue(Uuid, TheValue),
    ScrollBy(TheId, Vec2<i32>),

    GainedFocus(TheId),
    LostFocus(TheId),
    GainedHover(TheId),
    LostHover(TheId),
    SizeChanged(TheId),

    RedirectWidgetValueToLayout(TheId, TheId, TheValue),

    SetStatusText(TheId, String),

    // Tabbar, Groupbutton
    IndexChanged(TheId, usize),

    // The index of the palette has changed.
    PaletteIndexChanged(TheId, u16),
    ColorButtonClicked(TheId),

    // Tile / Code Editor
    TileSelectionChanged(TheId),
    TilePicked(TheId, Vec2<i32>),
    TileDragStarted(TheId, Vec2<i32>, Vec2<i32>),
    TileEditorClicked(TheId, Vec2<i32>),
    TileEditorDragged(TheId, Vec2<i32>),
    TileEditorHoverChanged(TheId, Vec2<i32>),
    TileEditorDrop(TheId, Vec2<i32>, TheDrop),
    TileEditorDelete(TheId, FxHashSet<(i32, i32)>),
    TileEditorUp(TheId),
    TileZoomBy(TheId, f32),

    RenderViewClicked(TheId, Vec2<i32>),
    RenderViewDragged(TheId, Vec2<i32>),
    RenderViewHoverChanged(TheId, Vec2<i32>),
    RenderViewLostHover(TheId),
    RenderViewScrollBy(TheId, Vec2<i32>),
    RenderViewUp(TheId, Vec2<i32>),
    RenderViewDrop(TheId, Vec2<i32>, TheDrop),
    RenderViewContext(TheId, Vec2<i32>),

    // Timeline
    TimelineMarkerSelected(TheId, TheTime),

    // SDF
    SDFIndexChanged(TheId, u32),

    // Show the given context menu at the given (global) coordinates.
    ShowContextMenu(TheId, Vec2<i32>, TheContextMenu),
    ShowMenu(TheId, Vec2<i32>, TheContextMenu),
    ContextMenuSelected(TheId, TheId),
    ContextMenuClosed(TheId),

    // Nodes
    NodeSelectedIndexChanged(TheId, Option<usize>),
    NodeDragged(TheId, usize, Vec2<i32>),
    NodeConnectionAdded(TheId, Vec<(u16, u8, u16, u8)>),
    NodeConnectionRemoved(TheId, Vec<(u16, u8, u16, u8)>),
    NodeDeleted(TheId, usize, Vec<(u16, u8, u16, u8)>),
    NodeViewScrolled(TheId, Vec2<i32>),

    //
    DialogValueOnClose(TheDialogButtonRole, String, Uuid, TheValue),

    // These events define layout states.
    SetStackIndex(TheId, usize),
    NewListItemSelected(TheId, TheId),
    ScrollLayout(TheId, Vec2<i32>),
    SnapperStateChanged(TheId, TheId, bool),
    TreeOpenStateChanged(TheId, bool),

    // Utility
    FileRequesterResult(TheId, Vec<std::path::PathBuf>),
    ImageDecodeResult(TheId, String, TheRGBABuffer),
    ExternalUrlRequested(String),

    // The top canvas has been resized.
    Resize,
    // A widget has been resized,
    WidgetResized(TheId, TheDim),

    // --- Clipboard

    // Sets the clipboard with the value and an optional app specific description.
    // For example to specify a certain type of JSON content.
    SetClipboard(TheValue, Option<String>),
    // Events send to Widgets to handle Cut / Copy / Paste events.
    // They can set the clipboard via the SetClipboard event above.
    Cut,
    Copy,
    Paste(TheValue, Option<String>),
    // Send when the content of the Clipboard changed.
    ClipboardChanged,

    // Undo / Redo, mostly only for TheTextAreaEdit.
    Undo,
    Redo,

    // Custom event for applications.
    Custom(TheId, TheValue),

    // Custom Undo Event
    CustomUndo(TheId, String, String),
}
