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
