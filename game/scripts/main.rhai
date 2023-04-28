import "button" as button;

fn init() {

  this.tile_size = 41;  // Default tile size
  this.gold = rgb(200, 200, 150);
  this.silver = rgb(192, 192, 192);
  this.tm1 = this.tilemaps.get("UIParts1");
  this.iconstm = this.tilemaps.get("Icons");
  this.movement_id = "";
  this.movement_visible = 0;
  this.font_name = "Roboto-Medium";
  this.context = "Inventory";
  this.board = rect(671, 197, 326, 364);
  this.items = rect(671, 90, 326, 80);
  this.item_id = -1;
  this.item_highlighted = 0;
  set_valid_mouse_rect(rect(80, 80, 510, 340));

  this.switch_button = button::TextButton();
  this.switch_button.text = "MODERN";
  this.switch_button.rect = rect(28, 28, 80, 20);

  this.region_mode_rect = rect(578, 28, 60, 20);
  this.region_mode = "2D";

  this.message.status("Welcome to the Eldiron Demo worthy adventurer.");

  // The action icon rectangles

  let ts = this.tile_size;
  this.actions = #{
    "Attack"      : rect(168, 505, 32, 32),
    "Move"        : rect(200, 505, 32, 32),
    "Look"        : rect(232, 505, 32, 32),
    "Use"         : rect(264, 505, 32, 32),
    "Talk"        : rect(296, 505, 32, 32),
    "Equip"       : rect(328, 505, 32, 32),
    "Take"        : rect(360, 505, 32, 32),
    "Drop"        : rect(392, 505, 32, 32),

    "Weapons"     : rect(464, 505, 32, 32),
    "Gear"        : rect(496, 505, 32, 32),
    "Inventory"   : rect(528, 505, 32, 32),
    "Spells"      : rect(560, 505, 32, 32),
  };
  this.curr_action = "Move";

  this.action_text = "FOR MORE INFORMATION PLEASE VISIT ELDIRON.COM";
  this.context_text = "";
}

// Draw the screen
fn draw() {
  let ts = this.tile_size;

  let shape = shape();
  let s = 25; let game_width = 640; let game_height = 581; shape.border_size = 2.0;
  if this.switch_button.text == "CLASSIC" {
    shape.color = rgb(20, 30, 100); shape.border_color = rgb(100, 100, 100);
    shape.add_rounded_rect(rect(0, 0, 200, s), rect(10, 15, 0, 15));                     // Top Left
    shape.add_rounded_rect(rect(400, 0, this.width - 400, s), rect(15, 10, 15, 0));      // Top Right
    shape.add_rounded_rect(rect(0, 0, s, game_height), rect(10, 0, 10, 0));              // Left Vertical
    shape.add_rounded_rect(rect(game_width, 0, s, game_height), rect(10, 10, 0, 10));    // Middle Vertical
    shape.add_rounded_rect(rect(0, game_height - s, this.width, s), rect(0, 0, 10, 10));
    shape.add_rect(rect(game_width, 70, this.width - game_width, 20));
    shape.add_rect(rect(game_width, 170, this.width - game_width, s));
    shape.add_rounded_rect(rect(this.width - s, 0, s, game_height), rect(0, 10, 0, 10));
    draw_shape(shape);
  } else {
    draw_rect(rect(0, 0, this.width, this.height), rgb(0, 0, 0));
  }

  // Game

  // 3d ?
  if this.region.supports_3d == true && this.region_mode == "3D" {
    draw_game_3d(rect(s, s, 615, 533));
    draw_game_2d(rect(s + 615 - 150, s, 150, 150));
    this.region.display_mode = "3d";
  } else {
    if this.switch_button.text == "CLASSIC" {
      draw_game_2d(rect(s, s, 615, 533));
    } else {
      draw_game_offset_2d(rect(15, 10, this.width - 35, this.height - 34), pos(5, 0));
    }
    this.region.display_mode = "2d";
  }

  draw_rect(rect(215, 2, 180, 25), rgb(0, 0, 0));
  draw_text(pos(220, 2), "Eldiron Demo " + this.date.time12(), this.font_name, 19.0, rgb(150, 150, 150));

  // Buttons

  this.switch_button.draw();

  if this.region.supports_3d == true {
    // This region supports 3d, make it switchable
    draw_rect(this.region_mode_rect, rgb(0, 0, 0));
    draw_text_rect(this.region_mode_rect, this.region_mode, this.font_name, 15.0, rgb(180, 180, 180), "center");
  }

  // Movement control
  let offset = 0; if this.movement_id == "West" { offset = 1; };
  draw_tile_sat(pos(1 * ts, 11 * ts), this.tm1.get_tile(1 + offset, 8), this.silver);
  offset = 0; if this.movement_id == "North" { offset = 1; };
  draw_tile_sat(pos(2 * ts, 10 * ts), this.tm1.get_tile(7 + offset, 6), this.silver);
  offset = 0; if this.movement_id == "East" { offset = 1; };
  draw_tile_sat(pos(3 * ts, 11 * ts), this.tm1.get_tile(4 + offset, 8), this.silver);
  offset = 0; if this.movement_id == "South" { offset = 1; };
  draw_tile_sat(pos(2 * ts, 12 * ts), this.tm1.get_tile(7 + offset, 7), this.silver);
  this.movement_visible -= 1;
  if this.movement_visible <= 0 { this.movement_id = ""; }

  // Action Icons
  let tile;
  if this.curr_action == "Attack" { tile = this.iconstm.get_tile(23, 1); } else { tile = this.iconstm.get_tile(22, 1); }
  draw_tile_sized(this.actions.Attack.pos, tile, 32);
  if this.curr_action == "Move" { tile = this.iconstm.get_tile(3, 4); } else { tile = this.iconstm.get_tile(2, 4); }
  draw_tile_sized(this.actions.Move.pos, tile, 32);
  if this.curr_action == "Look" { tile = this.iconstm.get_tile(1, 0);} else { tile = this.iconstm.get_tile(0, 0); }
  draw_tile_sized(this.actions.Look.pos, tile, 32);
  if this.curr_action == "Use" { tile = this.iconstm.get_tile(21, 2);} else { tile = this.iconstm.get_tile(20, 2); }
  draw_tile_sized(this.actions.Use.pos, tile, 32);
  if this.curr_action == "Talk" { tile = this.iconstm.get_tile(7, 0);} else { tile = this.iconstm.get_tile(6, 0); }
  draw_tile_sized(this.actions.Talk.pos, tile, 32);
  if this.curr_action == "Equip" { tile = this.iconstm.get_tile(21, 1);} else { tile = this.iconstm.get_tile(20, 1); }
  draw_tile_sized(this.actions.Equip.pos, tile, 32);
  if this.curr_action == "Take" { tile = this.iconstm.get_tile(13, 2);} else { tile = this.iconstm.get_tile(12, 2); }
  draw_tile_sized(this.actions.Take.pos, tile, 32);
  if this.curr_action == "Drop" { tile = this.iconstm.get_tile(15, 2);} else { tile = this.iconstm.get_tile(14, 2); }
  draw_tile_sized(this.actions.Drop.pos, tile, 32);
  if this.context == "Spells" { tile = this.iconstm.get_tile(25, 2);} else { tile = this.iconstm.get_tile(24, 2); }
  draw_tile_sized(this.actions.Spells.pos, tile, 32);
  if this.context == "Weapons" { tile = this.iconstm.get_tile(23, 1);} else { tile = this.iconstm.get_tile(22, 1); }
  draw_tile_sized(this.actions.Weapons.pos, tile, 32);
  if this.context == "Gear" { tile = this.iconstm.get_tile(23, 0);} else { tile = this.iconstm.get_tile(22, 0); }
  draw_tile_sized(this.actions.Gear.pos, tile, 32);
  if this.context == "Inventory" { tile = this.iconstm.get_tile(11, 0);} else { tile = this.iconstm.get_tile(10, 0); }
  draw_tile_sized(this.actions.Inventory.pos, tile, 32);

  // Player Info
  if this.switch_button.text == "MODERN" {
    draw_rect(rect(671 - 8, 26 - 8, 326 + 16, 40 + 16), rgba(20, 20, 20, 220));
    draw_rect(rect(671 - 8, 90 - 8, 326 + 16, 80 + 16), rgba(20, 20, 20, 220));
    draw_rect(rect(671 - 8, 197 - 8, 326 + 16, 364 + 16), rgba(20, 20, 20, 220));
  }
  draw_text_rect(rect(671, 26, 326, 20), "" + this.player.name, this.font_name, 19.0, rgb(150, 150, 150), "left");
  draw_text_rect(rect(671, 26, 326, 20), "" + this.player.gold + "G", this.font_name, 19.0, rgb(150, 150, 150), "right");
  draw_text_rect(rect(671, 50, 326, 20), "HP: " + this.player.HP + "  STR:" + this.player.STR, this.font_name, 19.0, rgb(150, 150, 150), "left");
  draw_text_rect(rect(671, 50, 326, 20), "LVL: " + this.experience.level, this.font_name, 19.0, rgb(150, 150, 150), "right");

  // Messages
  draw_messages(this.board, this.font_name, 16.0, this.silver);

  if this.context == "Spells" {
    let y = 90;
    let color = rgb(150, 150, 150);
    let index = 0;
    for spell in this.spells {
      //if this.item_id == item && this.item_highlight > 0 {
      //  color = this.silver;
      //  this.item_highlight -= 1;
      //}
      draw_text(pos(671, y), "" + (index + 1) + ". " + spell.name, this.font_name, 19.0, color);
      //this.cmd.draw_text_rect(rect(671, y, 326, 20), "" + this.inventory.item_amount_at(item), this.font_name, 19.0, color, "right");
      y += 20;
      index += 1;
    }
  } else
  if this.context == "Inventory" {
    let y = 90;
    let color = rgb(150, 150, 150);
    for item in 0..this.inventory.len() {
      if this.item_id == item && this.item_highlight > 0 {
        color = this.silver;
        this.item_highlight -= 1;
      }
      draw_text(pos(671, y), "" + (item + 1) + ". " + this.inventory.item_name_at(item), this.font_name, 19.0, color);
      draw_text_rect(rect(671, y, 326, 20), "" + this.inventory.item_amount_at(item), this.font_name, 19.0, color, "right");
      y += 20;
    }
  } else
  if this.context == "Weapons" {
    let y = 90;
    let color = rgb(150, 150, 150);
    let name = this.weapons.item_name_for("main hand");
    if name == "" { name = "<Empty>"; };
    draw_text(pos(671, y), "Main Hand: " + name, this.font_name, 19.0, color);
  }

  // Action Text
  draw_text(pos(10, this.height - 22), this.action_text, this.font_name, 14.0, rgb(150, 150, 150));
  if this.context_text.len() > 0 {
    draw_text(pos(300, this.height - 22), this.context_text, this.font_name, 14.0, rgb(150, 150, 150));
  }
}

// Handle the mouse events
fn touch_down(x, y) {

  if this.switch_button.clicked(pos(x, y)) {
    if this.switch_button.text == "CLASSIC" {
      this.switch_button.text = "MODERN";
    } else {
      this.switch_button.text = "CLASSIC";
    }
    return;
  }

  if this.region_mode_rect.is_inside(pos(x,y)) {
    if this.region_mode == "2D" {
      this.region_mode = "3D";
    } else {
      this.region_mode = "2D";
    }
    return;
  }

  let tx = x / this.tile_size; let ty = y / this.tile_size;

  let dir = "";
  if tx == 1 && ty == 11 {
    dir = "West"
  } else
  if tx == 2 && ty == 10 {
    dir = "North";
  } else
  if tx == 3 && ty == 11 {
    dir = "East";
  } else
  if tx == 2 && ty == 12 {
    dir = "South";
  }

  if dir.len() > 0 {
    this.handle_action(dir);
    return;
  }

  for item in this.actions.keys() {
    if this.actions[item].is_inside(pos(x, y) ) {
      if item == "Spells" || item == "Messages" || item == "Gear" || item == "Inventory" || item == "Weapons" {
        this.context = item;
      } else {
        this.curr_action = item;
        this.set_info_text();
      }
    }
  }

  // Item based action ?
  if this.items.is_inside(pos(x, y)) {
    let item = (y - this.items.y) / 20;
    if this.inventory.len() > item {
      action_inventory(this.curr_action, item);
      this.item_id = 0;
      this.item_highlight = 2;
      this.curr_action = "Move";
    }
  }

  // Coordinate based action ?
  action_coordinate(this.curr_action);
  this.set_info_text();
}

// Handle key down events
fn key_down(key) {
  let dir = "";
  if key == "left" {
    dir = "West";
  } else
  if key == "right" {
    dir = "East";
  } else
  if key == "up" {
    dir = "North";
  } else
  if key == "down" {
    dir = "South";
  } else
  if key == "a" {
    this.curr_action = "Attack";
  } else
  if key == "l" {
    this.curr_action = "Look";
  } else
  if key == "k" {
    this.curr_action = "Talk";
  } else
  if key == "m" {
    this.curr_action = "Move";
  } else
  if key == "u" {
    this.curr_action = "Use";
  } else
  if key == "t" {
    this.curr_action = "Take";
  } else
  if key == "d" {
    this.curr_action = "Drop";
  } else
  if key == "k" {
    this.curr_action = "Talk";
  } else
  if key == "i" {
    this.context = "Inventory";
  } else
  if key == "g" {
    this.context = "Gear";
  } else
  if key == "w" {
    this.context = "Weapons";
  } else
  if key == "s" {
    this.context = "Spells";
  } else
  if key == "e" {
    this.curr_action = "Equip";
  }

  // Inventory Action ?
  if this.context == "Inventory" {
    try {
      let key_int = parse_int(key) - 1;
      if key_int >= 0 {
        if this.inventory.len() > key_int {
          action_inventory(this.curr_action, key_int);
          this.item_id = key_int;
          this.item_highlight = 2;
          this.curr_action = "Move";
          return;
        }
      }
    }
    catch(err) {}
  }

  this.set_info_text();
  this.handle_action(dir);
}

// Execute action for the given direction
fn handle_action(dir) {
  if dir.len() > 0 {
    this.movement_id = dir;
    this.movement_visible = 2;
    action(this.curr_action, dir);
    this.curr_action = "Move";
  }
}

// Set info text
fn set_info_text() {

  this.action_text = "Action: ";
  if this.curr_action == "Move" {
    this.action_text += "Move (M)";
  } else
  if this.curr_action == "Attack" {
    this.action_text += "Attack (A)";
  } else
  if this.curr_action == "Look" {
    this.action_text += "Look (L)";
  } else
  if this.curr_action == "Talk" {
    this.action_text += "Talk (K)";
  } else
  if this.curr_action == "Use" {
    this.action_text += "Use (U)";
  } else
  if this.curr_action == "Take" {
    this.action_text += "Take (T)";
  } else
  if this.curr_action == "Drop" {
    this.action_text += "Drop (D)";
  } else
  if this.curr_action == "Equip" {
    this.action_text += "Equip (E)";
  }

  this.context_text = "Context: ";
  if this.context == "Spells" {
    this.context_text += "Spells (S)";
  } else
  if this.context == "Inventory" {
    this.context_text += "Inventory (I)";
  } else
  if this.context == "Gear" {
    this.context_text += "Gear (G)";
  } else
  if this.context == "Weapons" {
    this.context_text += "Weapons (W)";
  }
}
