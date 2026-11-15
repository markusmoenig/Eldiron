//
//  Header.h
//  Xcode2Rust
//
//  Created by Markus Moenig on 30/10/22.
//

#ifndef Bridge_h
#define Bridge_h

#import "Metal.h"

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define GAME_TICK_IN_MS 250

// Version 1: independent main-thread Creator owners, one presenter per layer.
// Handles are monotonic per thread and never recycled. Zero means failure.
// Destroy the Rust host before releasing its retained CAMetalLayer.
// Stale handles fail; every asynchronous completion must capture its original handle.
// Unversioned callbacks address only a separately rust_init-created legacy host.
uint32_t eldiron_creator_abi_version(void);
uint64_t eldiron_creator_create_v1(uint32_t version, void *layer, uint32_t width, uint32_t height, float density);
bool eldiron_creator_destroy_v1(uint64_t host);
int32_t eldiron_creator_render_v1(uint64_t host);
bool eldiron_creator_replace_layer_v1(uint64_t host, void *layer);
bool eldiron_creator_resize_v1(uint64_t host, uint32_t width, uint32_t height, float density);
bool eldiron_creator_suspend_v1(uint64_t host);
bool eldiron_creator_resume_v1(uint64_t host);
bool eldiron_creator_focus_v1(uint64_t host, bool focused);
char *eldiron_creator_state_v1(uint64_t host);
char *eldiron_creator_requests_v1(uint64_t host);
char *eldiron_creator_text_snapshot_v1(uint64_t host);
char *eldiron_creator_text_query_v1(uint64_t host, uint64_t target, uint64_t revision, uint32_t kind, intptr_t start, intptr_t end, float x, float y);
int32_t eldiron_creator_text_edit_v1(uint64_t host, uint64_t target, uint64_t revision, uint32_t kind, intptr_t start, intptr_t end, const uint8_t *data, uintptr_t length, intptr_t cursor_start, intptr_t cursor_end);
bool eldiron_creator_key_v1(uint64_t host, uint32_t code, bool pressed, bool repeated, const char *logical);
bool eldiron_creator_clipboard_complete_v1(uint64_t host, uint64_t request, bool write, bool success, const char *text);
// Query/copy an owned PNG. Undersized output is untouched; stale requests return 0.
uintptr_t eldiron_creator_clipboard_image_copy_v1(uint64_t host, uint64_t request, uint8_t *out, uintptr_t capacity);
bool eldiron_creator_clipboard_image_complete_v1(uint64_t host, uint64_t request, bool write, bool success, const uint8_t *png, uintptr_t length);
bool eldiron_creator_edit_command_v1(uint64_t host, uint32_t command);
bool eldiron_creator_has_changes_v1(uint64_t host);
// Commands 0..17: New, Open, Close, Save, Save As, Play, Pause, Stop,
// Settings, Rules, Locales, Audio FX, Authoring, Project, Actions, Log, Console, Help.
bool eldiron_creator_command_v1(uint64_t host, uint32_t command);
bool eldiron_creator_set_theme_v1(uint64_t host, uint32_t theme);
uint32_t eldiron_creator_get_theme_v1(uint64_t host);
bool eldiron_creator_drop_file_v1(uint64_t host, const char *path);
bool eldiron_creator_hover_v1(uint64_t host, float x, float y);
bool eldiron_creator_touch_down_v1(uint64_t host, float x, float y);
bool eldiron_creator_touch_up_v1(uint64_t host, float x, float y);
bool eldiron_creator_touch_dragged_v1(uint64_t host, float x, float y);
bool eldiron_creator_right_touch_down_v1(uint64_t host, float x, float y);
bool eldiron_creator_right_touch_up_v1(uint64_t host, float x, float y);
bool eldiron_creator_right_touch_dragged_v1(uint64_t host, float x, float y);
bool eldiron_creator_mouse_motion_v1(uint64_t host, float x, float y);
bool eldiron_creator_mouse_leave_v1(uint64_t host);
bool eldiron_creator_scroll_v1(uint64_t host, float x, float y);
bool eldiron_creator_precise_scroll_v1(uint64_t host, float x, float y);
bool eldiron_creator_pinch_v1(uint64_t host, float delta);
bool eldiron_creator_modifiers_v1(uint64_t host, bool shift, bool control, bool alt, bool super_key);
// Owned UTF-8 JSON strings: free with rust_free_string. Null = invalid host.
// Document responses: {"ok":true,"value":...} or {"ok":false,"error":"..."}.
// expected is the active document UUID returned by document_info, captured before I/O.
// Load validates/decode before mutation. replace_clean rejects a dirty active tab.
char *eldiron_creator_document_info_v1(uint64_t host);
char *eldiron_creator_document_load_v1(uint64_t host, const char *expected, const uint8_t *data, uintptr_t length, const char *origin, bool replace_clean);
// Prepare returns {ticket,document,length}; only one immutable snapshot can be pending.
char *eldiron_creator_document_prepare_save_v1(uint64_t host, const char *expected);
// Null destination/capacity 0 queries length. Undersized buffers are untouched and
// return required length. Copy owns no caller memory after return. Zero = stale ticket.
uintptr_t eldiron_creator_document_copy_save_v1(uint64_t host, uint64_t ticket, uint8_t *destination, uintptr_t capacity);
// Complete exactly once, including cancel/failure. Saved != clean if edits overlapped.
char *eldiron_creator_document_finish_save_v1(uint64_t host, uint64_t ticket, bool success, const char *origin);

// Legacy compatibility exports; new wrappers use the handle-scoped API above.
// Main-thread calls. Retain CAMetalLayer until rust_shutdown/replacement returns.
// Dimensions are physical pixels; pointer coordinates are logical points.
// Swift never acquires or presents this layer's drawables.
bool rust_init(void *layer, uint32_t width, uint32_t height, float density);
void rust_shutdown(void);
int32_t rust_render(void);
bool rust_replace_layer(void *layer);
void rust_resize(uint32_t width, uint32_t height, float density);
void rust_suspend(void);
void rust_resume(void);
void rust_focus(bool focused);
char *rust_take_requests(void);
char *rust_host_state(void);
char *rust_text_snapshot(void);
char *rust_text_query(uint64_t target, uint64_t revision, uint32_t kind, intptr_t start, intptr_t end, float x, float y);
int32_t rust_text_edit(uint64_t target, uint64_t revision, uint32_t kind, intptr_t start, intptr_t end, const uint8_t *data, uintptr_t length, intptr_t cursor_start, intptr_t cursor_end);
bool rust_physical_key(uint32_t code, bool pressed, bool repeated, const char *logical);
void rust_free_string(char *text);
void rust_clipboard_complete(uint64_t request, bool write, bool success, const char *text);
void rust_edit_command(uint32_t command);
void rust_dropped_file(const char *path);
void rust_new();
void rust_play();
void rust_pause();
void rust_stop();

//void rust_init(const char *r, const char *p);

uint32_t rust_target_fps(void);

bool rust_hover(float x, float y);

bool rust_touch_down(float x, float y);

bool rust_right_touch_down(float x, float y);

bool rust_touch_dragged(float x, float y);

bool rust_right_touch_dragged(float x, float y);

bool rust_mouse_motion(float delta_x, float delta_y);
bool rust_mouse_leave(void);

bool rust_touch_up(float x, float y);

bool rust_right_touch_up(float x, float y);

bool rust_touch_wheel(float x, float y);
bool rust_touch_precise_scroll(float x, float y);
bool rust_pinch(float delta);

bool rust_key_modifier_changed(bool, bool, bool, bool);

void rust_open();
void rust_close();
void rust_save();
void rust_save_as();
void rust_set_creator_theme(uint32_t);
uint32_t rust_get_creator_theme();
void rust_show_settings();
void rust_show_rules();
void rust_show_locales();
void rust_show_audio_fx();
void rust_show_authoring();
void rust_show_project_sidebar();
void rust_show_actions_sidebar();
void rust_show_debug_log();
void rust_show_console();
void rust_show_help_sidebar();

void rust_undo();
void rust_redo();
bool rust_has_changes(void);

#endif /* Header_h */
