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

#define KEY_ESCAPE 0
#define KEY_RETURN 1
#define KEY_DELETE 2
#define KEY_UP 3
#define KEY_RIGHT 4
#define KEY_DOWN 5
#define KEY_LEFT 6
#define KEY_SPACE 7
#define KEY_TAB 8

void rust_init();
void rust_draw(uint8_t *pixels, uint32_t width, uint32_t height, uintptr_t anim_counter);
void rust_update();

//void rust_init(const char *r, const char *p);

uint32_t rust_target_fps(void);

bool rust_hover(float x, float y);

bool rust_touch_down(float x, float y);

bool rust_touch_dragged(float x, float y);

bool rust_touch_up(float x, float y);

bool rust_touch_wheel(float x, float y);

bool rust_key_down(const char *);
bool rust_special_key_down(uint32_t key);

void rust_open();

void rust_undo();
void rust_redo();

const char * rust_cut();
const char * rust_copy();
const char * rust_paste(const char *);


#endif /* Header_h */
