fn init() {
}

fn draw() {
  draw_text_rect(rect(0, 0, get_width(), get_height()), "Click to start Demo ...", "Roboto-Medium", 15.0, rgb(180, 180, 180), "center");
}

fn touch_down(x, y) {
  login_local_user("local", "Characters");
}