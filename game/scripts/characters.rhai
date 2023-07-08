import "button" as button;

fn init() {

    this.logo = get_images().get("eldiron_logo");
}

// Draw the screen
fn draw() {

  draw_image(pos(350, 180), this.logo, 300, 300, 0.01);
}

// Handle the mouse events
fn touch_down(x, y) {
}

fn key_down(key) {
}