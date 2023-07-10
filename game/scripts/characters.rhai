import "button" as button;

fn init() {

    this.logo = get_images().get("eldiron_logo");

    this.new_char_name = button::TextEntry("New Character", rect(250, 100, 200, 20));
    this.create_char_button = button::TextButton("CREATE CHARACTER", rect(250, 160, 160, 20));

}

// Draw the screen
fn draw() {

  draw_image(pos(350, 180), this.logo, 300, 300, 0.01);

  draw_text_rect(rect(140, 100, 100, 20), "NAME", "Roboto-Medium", 15.0, rgb(180, 180, 180), "right");

  this.new_char_name.draw();
  this.create_char_button.draw();
}

// Handle the mouse events
fn touch_down(x, y) {

  if this.new_char_name.clicked(pos(x, y)) {

  } else
  if this.create_char_button.clicked(pos(x, y)) {
    enter_game_and_create_character(this.new_char_name.text, "", "", "Game");
  }
}

fn key_down(key) {
  this.new_char_name.key_down(key);
}