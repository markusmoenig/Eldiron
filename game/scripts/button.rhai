
fn TextButton() {
  let button = #{
    text: "Button",
    rect: rect(0, 0, 100, 40),
  };
  button.draw = || {
    draw_rect(this.rect, rgb(255, 255, 255));
  };
  button
}