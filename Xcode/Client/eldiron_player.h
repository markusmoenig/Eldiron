#ifndef ELDIRON_PLAYER_H
#define ELDIRON_PLAYER_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Opaque runner. All calls are serialized on its creation/main thread. Swift
 * retains the CAMetalLayer through destroy/replace_layer returning. Rust creates
 * and owns the WGPU surface/device and exclusively acquires/presents drawables.
 * Resize sizes are physical pixels; pointer/caret positions are logical points.
 * A document keeps its runner across view recreation and replaces only its layer.
 */
typedef struct EldironPlayerRunner EldironPlayerRunner;
EldironPlayerRunner *unified_app_runner_create(void *layer, uint32_t width, uint32_t height, float scale);
void unified_app_runner_destroy(EldironPlayerRunner *);
void unified_app_runner_resize(EldironPlayerRunner *, uint32_t width, uint32_t height, float scale);
/* Render: 0 success/unchanged, 1 deferred/recovered, -1 error. Only actual
 * presentations increment host_state.presentations; updates do not require one. */
int32_t unified_app_runner_render(EldironPlayerRunner *);
void unified_app_runner_suspend(EldironPlayerRunner *);
void unified_app_runner_resume(EldironPlayerRunner *);
int32_t unified_app_runner_replace_layer(EldironPlayerRunner *, void *layer, uint32_t width, uint32_t height, float scale);
/* Input bytes are borrowed for the call. Project data accepts legacy JSON or
 * Eldiron ZIP archives. Saving returns the exact loaded bytes, never runtime
 * mutations. Existing *_json names do not imply a text-only project format. */
int32_t unified_app_runner_load_project(EldironPlayerRunner *, const uint8_t *, size_t);
int32_t unified_app_runner_save_project(EldironPlayerRunner *, const uint8_t **, size_t *);
/* Each successful output is Rust-owned, immutable bytes. Free it exactly once
 * with the original pointer/length. Null/zero outputs may also be freed. */
void unified_app_runner_free_json(const uint8_t *, size_t);
void unified_app_runner_free_data(const uint8_t *, size_t);
void unified_app_runner_free_string(const uint8_t *, size_t);
int32_t unified_app_runner_last_error(EldironPlayerRunner *, const uint8_t **, size_t *);
/* UTF-8 JSON host state and drained addressed platform requests. Clipboard
 * completions carry the original request ID; text/preedit carry the target token
 * from the native input session that began the operation (never retarget later).
 * Returned input status is 1 consumed, 0 unconsumed/stale, -1 error. */
int32_t unified_app_runner_host_state(EldironPlayerRunner *, const uint8_t **, size_t *);
int32_t unified_app_runner_take_requests(EldironPlayerRunner *, const uint8_t **, size_t *);
int32_t unified_app_runner_text(EldironPlayerRunner *, uint64_t target, const uint8_t *, size_t);
/* Composition cursor endpoints are UTF-8 byte offsets, or -1 for no selection. */
int32_t unified_app_runner_composition(EldironPlayerRunner *, uint64_t target, const uint8_t *, size_t, intptr_t start, intptr_t end);
int32_t unified_app_runner_clipboard(EldironPlayerRunner *, uint64_t request, int32_t write, int32_t success, const uint8_t *, size_t);
void unified_app_runner_focus(EldironPlayerRunner *, int32_t focused);
/* Modifier bits: shift=1, control=2, alt=4, super/command=8. */
void unified_app_runner_modifiers(EldironPlayerRunner *, uint32_t bits);
/* Keys: ASCII letters/digits for physical controls, not committed text. */
enum EldironKey {
    ELDIRON_ENTER=0x100, ELDIRON_ESCAPE, ELDIRON_BACKSPACE, ELDIRON_TAB,
    ELDIRON_SPACE, ELDIRON_DELETE, ELDIRON_LEFT, ELDIRON_RIGHT, ELDIRON_UP,
    ELDIRON_DOWN, ELDIRON_HOME, ELDIRON_END, ELDIRON_PAGE_UP, ELDIRON_PAGE_DOWN,
    ELDIRON_SHIFT_LEFT=0x110, ELDIRON_SHIFT_RIGHT, ELDIRON_CONTROL_LEFT,
    ELDIRON_CONTROL_RIGHT, ELDIRON_ALT_LEFT, ELDIRON_ALT_RIGHT,
    ELDIRON_SUPER_LEFT, ELDIRON_SUPER_RIGHT,
    ELDIRON_F1=0x201, ELDIRON_F24=0x218
};
int32_t unified_app_runner_key(EldironPlayerRunner *, uint32_t physical, int32_t pressed, int32_t repeated);
/* Buttons: primary=0, secondary=1, middle=2; other values are passed through. */
int32_t unified_app_runner_pointer_button(EldironPlayerRunner *, uint16_t button, int32_t pressed, float x, float y);
void unified_app_runner_mouse_down(EldironPlayerRunner *, float x, float y);
void unified_app_runner_mouse_up(EldironPlayerRunner *, float x, float y);
void unified_app_runner_mouse_move(EldironPlayerRunner *, float x, float y);
void unified_app_runner_mouse_motion(EldironPlayerRunner *, float dx, float dy);
void unified_app_runner_mouse_leave(EldironPlayerRunner *);
void unified_app_runner_scroll(EldironPlayerRunner *, float dx, float dy);
void unified_app_runner_pinch(EldironPlayerRunner *, float scale, float x, float y);
#ifdef __cplusplus
}
#endif
#endif
