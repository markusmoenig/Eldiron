#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define KEY_ESCAPE 0

#define KEY_RETURN 1

#define KEY_DELETE 2

#define KEY_UP 3

#define KEY_RIGHT 4

#define KEY_DOWN 5

#define KEY_LEFT 6

#define KEY_SPACE 7

#define KEY_TAB 8

#define GAME_TICK_IN_MS 250

void rust_draw(uint8_t *pixels, uint32_t width, uint32_t height, uintptr_t anim_counter);

void rust_init(const char *p);

uint32_t rust_target_fps(void);

bool rust_hover(float x, float y);

bool rust_touch_down(float x, float y);

bool rust_touch_dragged(float x, float y);

bool rust_touch_up(float x, float y);

bool rust_touch_wheel(float x, float y);

bool rust_key_down(const char *p);

bool rust_special_key_down(uint32_t key);
