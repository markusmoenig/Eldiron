// A text button

fn TextButton() {
  let button = #{
    text              : "Button",
    font_name         : "Roboto-Medium",
    font_size         : 15.0,
    rect              : rect(0, 0, 100, 40),
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