// A text button

fn TextButton(text, rect) {
  let button = #{
    text              : text,
    font_name         : "Roboto-Medium",
    font_size         : 15.0,
    rect              : rect,
    background_color  : rgb(0, 0, 0),
    text_color        : rgb(180, 180, 180),
    clicked_count     : -1,
  };

  button.draw = || {
    // Inverse the colors on click
    if this.clicked_count <= 0 {
      draw_rect(this.rect, this.background_color);
      draw_text_rect(this.rect, this.text, this.font_name, this.font_size, this.text_color, "center");
    } else {
      draw_rect(this.rect, this.text_color);
      draw_text_rect(this.rect, this.text, this.font_name, this.font_size, this.background_color, "center");
    }

    if this.clicked_count > 0 {
      this.clicked_count -= 1;
    }
  };

  button.clicked = |pos| {
    if this.rect.is_inside(pos) {
      this.clicked_count = 2;
      true
    } else {
      false
    }
  };

  button
}

// A text entry field

fn TextEntry(text, rect) {
  let button = #{
    text              : text,
    font_name         : "Roboto-Medium",
    font_size         : 15.0,
    rect              : rect,
    background_color  : rgb(20, 20, 20),
    focus_color       : rgb(60, 60, 60),
    text_color        : rgb(180, 180, 180),
    has_focus         : false,
  };

  button.draw = || {
    if this.has_focus {
      draw_rect(this.rect, this.focus_color);
    } else {
      draw_rect(this.rect, this.background_color);
    }
    draw_text_rect(this.rect, this.text, this.font_name, this.font_size, this.text_color, "left");
  };

  button.clicked = |pos| {
    if this.rect.is_inside(pos) {
      this.has_focus = true;
      true
    } else {
      false
    }
  };

  button.key_down = |key| {
    if this.has_focus {
      if key == "DEL" {
        let len = this.text.len();
        if len > 0 {
          this.text = this.text.sub_string(0, len - 1);
        }
      } else {
        if this.text.len() < 20 {
          this.text += key;
        }
      }
    }
  };

  button
}

// A text display

fn TextDisplay(text, rect) {
  let button = #{
    text              : text,
    font_name         : "Roboto-Medium",
    font_size         : 15.0,
    rect              : rect,
    background_color  : rgb(0, 0, 0),
    text_color        : rgb(180, 0, 0),
    has_focus         : false,
  };

  button.draw = || {
    draw_rect(this.rect, this.background_color);
    draw_text_rect(this.rect, this.text, this.font_name, this.font_size, this.text_color, "left");
  };

  button
}