import "button" as button;

fn init() {

    this.logo = get_images().get("eldiron_logo");

    this.login_user_entry = button::TextEntry("", rect(250, 100, 200, 20));
    this.login_password_entry = button::TextEntry("", rect(250, 125, 200, 20));
    this.login_button = button::TextButton("LOGIN", rect(250, 160, 50, 20));

    this.register_user_entry = button::TextEntry("", rect(650, 100, 200, 20));
    this.register_password_entry = button::TextEntry("", rect(650, 125, 200, 20));
    this.register_password_entry2 = button::TextEntry("", rect(650, 150, 200, 20));
    this.register_button = button::TextButton("REGISTER", rect(650, 180, 80, 20));

    this.error_text = button::TextDisplay("", rect(50, 550, 500, 20));
    this.enter_game_button = button::TextButton("ENTER AS ANONYMOUS", rect(650, 500, 200, 20));
}

// Draw the screen
fn draw() {

  draw_image(pos(350, 180), this.logo, 300, 300, 0.01);

  draw_text_rect(rect(140, 100, 100, 20), "USER", "Roboto-Medium", 15.0, rgb(180, 180, 180), "right");
  draw_text_rect(rect(140, 125, 100, 20), "PASSWORD", "Roboto-Medium", 15.0, rgb(180, 180, 180), "right");

  draw_text_rect(rect(540, 100, 100, 20), "USER", "Roboto-Medium", 15.0, rgb(180, 180, 180), "right");
  draw_text_rect(rect(540, 125, 100, 20), "PASSWORD", "Roboto-Medium", 15.0, rgb(180, 180, 180), "right");
  draw_text_rect(rect(540, 150, 100, 20), "VERIFY", "Roboto-Medium", 15.0, rgb(180, 180, 180), "right");

  this.login_user_entry.draw();
  this.login_password_entry.draw();
  this.login_button.draw();

  this.register_user_entry.draw();
  this.register_password_entry.draw();
  this.register_password_entry2.draw();
  this.register_button.draw();

  this.error_text.draw();
  this.enter_game_button.draw();
}

// Handle the mouse events
fn touch_down(x, y) {

  if this.login_user_entry.clicked(pos(x, y)) {
    this.login_password_entry.has_focus = false;

    this.register_user_entry.has_focus = false;
    this.register_password_entry.has_focus = false;
    this.register_password_entry2.has_focus = false;
  }

  if this.login_password_entry.clicked(pos(x, y)) {
    this.login_user_entry.has_focus = false;

    this.register_user_entry.has_focus = false;
    this.register_password_entry.has_focus = false;
    this.register_password_entry2.has_focus = false;
  }

  if this.register_user_entry.clicked(pos(x, y)) {
    this.login_user_entry.has_focus = false;
    this.login_password_entry.has_focus = false;

    this.register_password_entry.has_focus = false;
    this.register_password_entry2.has_focus = false;
  }

  if this.register_password_entry.clicked(pos(x, y)) {
    this.login_user_entry.has_focus = false;
    this.login_password_entry.has_focus = false;

    this.register_user_entry.has_focus = false;
    this.register_password_entry2.has_focus = false;
  }

  if this.register_password_entry2.clicked(pos(x, y)) {
    this.login_user_entry.has_focus = false;
    this.login_password_entry.has_focus = false;

    this.register_user_entry.has_focus = false;
    this.register_password_entry.has_focus = false;
  }

  if this.login_button.clicked(pos(x, y)) {
    // login
    if this.login_user_entry.text.len() == 0 {
      this.error_text.text = "USER NAME IS EMPTY";
    } else
    if this.login_password_entry.text.len() == 0 {
      this.error_text.text = "PASSWORD IS EMPTY";
    } else {
      login_user(this.login_user_entry.text, this.login_password_entry.text, "Game");
    }
  }

  if this.register_button.clicked(pos(x, y)) {
    // register
    if this.register_user_entry.text.len() == 0 {
      this.error_text.text = "USER NAME IS EMPTY";
    } else
    if this.register_password_entry.text.len() == 0 {
      this.error_text.text = "PASSWORD IS EMPTY";
    } else
    if this.register_password_entry.text != this.register_password_entry2.text {
      this.error_text.text = "PASSWORDS DO NOT MATCH";
    } else {
      register_user(this.register_user_entry.text, this.register_password_entry.text, "Game");
    }
  }

  if this.enter_game_button.clicked(pos(x, y)) {
    enter_game_and_create_character("Anonymous", "", "", "Game");
  }
}

fn key_down(key) {
  this.login_user_entry.key_down(key);
  this.login_password_entry.key_down(key);

  this.register_user_entry.key_down(key);
  this.register_password_entry.key_down(key);
  this.register_password_entry2.key_down(key);
}