import "button" as button;

fn init() {

    this.logo = get_images().get("eldiron_logo");

    this.new_char_name = button::TextEntry("New Character", rect(250, 100, 200, 20));
    this.create_char_button = button::TextButton("CREATE CHARACTER", rect(250, 160, 160, 20));

    this.login_char_button = button::TextButton("LOGIN CHARACTER", rect(700, 410, 160, 20));

    this.char_selection = "";
}

// Draw the screen
fn draw() {

  draw_image(pos(350, 180), this.logo, 300, 300, 0.01);

  draw_text_rect(rect(140, 100, 100, 20), "NAME", "Roboto-Medium", 15.0, rgb(180, 180, 180), "right");

  draw_rect_outline(rect(699, 99, 200, 300), rgb(180, 180, 180));

  this.new_char_name.draw();
  this.create_char_button.draw();

  let y = 100;
  let characters = get_characters();
  let index = 0;
  for c in characters {
    let rect = rect(700, y, 200, 20);
    if c.name == this.char_selection {
      draw_rect(rect, rgb(180, 180, 180));
      draw_text_rect(rect, c.name, "Roboto-Medium", 16.0, rgb(0, 0, 0), "left");
    } else {
      draw_text_rect(rect, c.name, "Roboto-Medium", 16.0, rgb(180, 180, 180), "left");
    }
    y += 20;
    index += 1;
  }

  this.login_char_button.draw();
}

// Handle the mouse events
fn touch_down(x, y) {

  if this.new_char_name.clicked(pos(x, y)) {

  } else
  if this.create_char_button.clicked(pos(x, y)) {
    enter_game_and_create_character(this.new_char_name.text, "", "", "Game");
  } else
  if this.login_char_button.clicked(pos(x, y)) {
    if this.char_selection != "" {
      enter_game_with_character(this.char_selection);
    }
  }

  let characters = get_characters();
  let y = 100;
  for c in characters {
    let rect = rect(700, y, 200, 20);
    if rect.is_inside(pos(x, y)) {
      this.char_selection = c.name;
    }
    y += 20;
  }
}

fn key_down(key) {
  this.new_char_name.key_down(key);
}