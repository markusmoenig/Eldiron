import "button" as button;

fn init() {

    this.player_name = "New Player";
    this.enter_game_button = button::TextButton("ENTER GAME", rect(600, 500, 100, 20));

}

// Draw the screen
fn draw() {
  this.enter_game_button.draw();

}

// Handle the mouse events
fn touch_down(x, y) {

  if this.enter_game_button.clicked(pos(x, y)) {
    enter_game_and_create_character(this.player_name, "", "", "Game");
  }
}

fn key_down(key) {

}