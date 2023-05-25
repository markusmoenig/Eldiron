// The spells widget

fn SpellsWidget(rect) {
  let spells = #{
    font_name         : "Roboto-Medium",
    font_size         : 19.0,
    rect              : rect,
    margin            : 8,
    text_color        : rgb(150, 150, 150),
    highlighted_color : rgb(192, 192, 192),
    selected_item     : -1,
    selected_name     : "",
    highlight_item    : -1,
  };

  spells.draw = || {
    let x = this.rect.x + this.margin;
    let y = this.rect.y + this.margin;
    let index = 0;
    let spells = get_spells();

    for spell in spells {
      let color = this.text_color;
      if this.selected_item == index {
        color = this.highlighted_color;
      }

      draw_tile_sized(pos(x, y - 2), spell.tile, 24);
      draw_text(pos(x + 26, y), "" + (index + 1) + ". " + spell.name, this.font_name, this.font_size, color);
      y += 20;
      index += 1;
    }
  };

  spells.key_down = |key| {
    try {
      let key_int = parse_int(key) - 1;
      if key_int >= 0 {
        let spells = get_spells();
        if spells.len() > key_int {
          this.selected_item = key_int;
          this.selected_name = spells.spell_at(this.selected_item).name;
          return true;
        }
      }
    }
    catch(err) {}
    return false;
  };

  spells.clicked = |pos| {
    if this.rect.is_inside(pos) {
      let y = this.rect.y + this.margin;
      let selected_item = (pos.y - y) / 20;
      let spells = get_spells();
      if spells.len() > selected_item {
        this.selected_item = selected_item;
        this.selected_name = spells.spell_at(selected_item).name;
      }
      true
    } else {
      false
    }
  };

  spells
}

// The inventory widget

fn InventoryWidget(rect) {
  let inventory = #{
    font_name         : "Roboto-Medium",
    font_size         : 19.0,
    rect              : rect,
    margin            : 8,
    text_color        : rgb(150, 150, 150),
    highlighted_color : rgb(192, 192, 192),
    selected_item     : -1,
    highlight_item    : -1,
  };

  inventory.draw = || {
    let x = this.rect.x + this.margin;
    let y = this.rect.y + this.margin;
    let index = 0;
    let inventory = get_inventory();

    for item in inventory {
      let color = this.text_color;
      if this.selected_item == index && this.highlight_item > 0 {
        color = this.highlighted_color;
        this.highlight_item -= 1;
      }

      draw_tile_sized(pos(x, y - 2), item.tile, 24);
      draw_text(pos(x + 26, y), "" + (index + 1) + ". " + item.name, this.font_name, this.font_size, color);
      draw_text_rect(rect(x, y, this.rect.width - 2 * this.margin, 20), "" + item.amount, this.font_name, this.font_size, color, "right");
      y += 20;
      index += 1;
    }
  };

  inventory.key_down = |key, curr_action| {
    try {
      let key_int = parse_int(key) - 1;
      if key_int >= 0 {
        let inventory = get_inventory();
        if inventory.len() > key_int {
          action_inventory(curr_action, key_int);
          this.selected_item = key_int;
          this.highlight_item = 2;
          return true;
        }
      }
    }
    catch(err) {}
    return false;
  };

  inventory.clicked = |pos, curr_action| {
    if this.rect.is_inside(pos) {
      let y = this.rect.y + this.margin;
      let selected_item = (pos.y - y) / 20;
      let inventory = get_inventory();
      if inventory.len() > selected_item {
        action_inventory(curr_action, selected_item);
        this.selected_item = selected_item;
        this.highlight_item = 2;
      }
      true
    } else {
      false
    }
  };

  inventory
}