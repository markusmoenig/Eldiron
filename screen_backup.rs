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

    "Weapon"      : rect(464, 505, 32, 32),
    "Gear"        : rect(496, 505, 32, 32),
    "Inventory"   : rect(528, 505, 32, 32),
  };
  this.curr_action = "Move";
}

// Draw the screen
fn draw() {
  let ts = this.tile_size;
  //this.cmd.draw_rect(rect(0, 0, this.width, this.height), rgb(0, 0, 0)); // Black background

  let shape = shape(); let s = 25; let game_width = 640; let game_height = 581; shape.border_size = 2.0;
  shape.color = rgb(20, 30, 100); shape.border_color = rgb(100, 100, 100);
  shape.add_rounded_rect(rect(0, 0, 200, s), rect(10, 15, 0, 15));                     // Top Left
  shape.add_rounded_rect(rect(400, 0, this.width - 400, s), rect(15, 10, 15, 0));      // Top Right
  shape.add_rounded_rect(rect(0, 0, s, game_height), rect(10, 0, 10, 0));              // Left Vertical
  shape.add_rounded_rect(rect(game_width, 0, s, game_height), rect(10, 10, 0, 10));    // Middle Vertical
  shape.add_rounded_rect(rect(0, game_height - s, this.width, s), rect(0, 0, 10, 10));
  shape.add_rect(rect(game_width, 70, this.width - game_width, 20));
  shape.add_rect(rect(game_width, 170, this.width - game_width, s));
  shape.add_rounded_rect(rect(this.width - s, 0, s, game_height), rect(0, 10, 0, 10));
  this.cmd.draw_shape(shape);

  this.cmd.draw_text(pos(220, 2), "Eldiron Demo v0.1", this.font_name, 19.0, rgb(150, 150, 150));

  // Game
  this.cmd.draw_game(rect(s, s, 615, 533));

  // Movement control
  let offset = 0; if this.movement_id == "West" { offset = 1; };
  this.cmd.draw_tile_sat(pos(1 * ts, 11 * ts), this.tm1.get_tile(1 + offset, 8), this.silver);
  offset = 0; if this.movement_id == "North" { offset = 1; };
  this.cmd.draw_tile_sat(pos(2 * ts, 10 * ts), this.tm1.get_tile(7 + offset, 6), this.silver);
  offset = 0; if this.movement_id == "East" { offset = 1; };
  this.cmd.draw_tile_sat(pos(3 * ts, 11 * ts), this.tm1.get_tile(4 + offset, 8), this.silver);
  offset = 0; if this.movement_id == "South" { offset = 1; };
  this.cmd.draw_tile_sat(pos(2 * ts, 12 * ts), this.tm1.get_tile(7 + offset, 7), this.silver);
  this.movement_visible -= 1;
  if this.movement_visible <= 0 { this.movement_id = ""; }

  // Action Icons
  let tile;
  if this.curr_action == "Attack" { tile = this.iconstm.get_tile(23, 1); } else { tile = this.iconstm.get_tile(22, 1); }
  this.cmd.draw_tile_sized(this.actions.Attack.pos, tile, 32);
  if this.curr_action == "Move" { tile = this.iconstm.get_tile(3, 4); } else { tile = this.iconstm.get_tile(2, 4); }
  this.cmd.draw_tile_sized(this.actions.Move.pos, tile, 32);
  if this.curr_action == "Look" { tile = this.iconstm.get_tile(1, 0);} else { tile = this.iconstm.get_tile(0, 0); }
  this.cmd.draw_tile_sized(this.actions.Look.pos, tile, 32);
  if this.curr_action == "Use" { tile = this.iconstm.get_tile(21, 2);} else { tile = this.iconstm.get_tile(20, 2); }
  this.cmd.draw_tile_sized(this.actions.Use.pos, tile, 32);
  if this.curr_action == "Talk" { tile = this.iconstm.get_tile(7, 0);} else { tile = this.iconstm.get_tile(6, 0); }
  this.cmd.draw_tile_sized(this.actions.Talk.pos, tile, 32);
  if this.curr_action == "Equip" { tile = this.iconstm.get_tile(21, 1);} else { tile = this.iconstm.get_tile(20, 1); }
  this.cmd.draw_tile_sized(this.actions.Equip.pos, tile, 32);
  if this.curr_action == "Take" { tile = this.iconstm.get_tile(13, 2);} else { tile = this.iconstm.get_tile(12, 2); }
  this.cmd.draw_tile_sized(this.actions.Take.pos, tile, 32);
  if this.curr_action == "Drop" { tile = this.iconstm.get_tile(15, 2);} else { tile = this.iconstm.get_tile(14, 2); }
  this.cmd.draw_tile_sized(this.actions.Drop.pos, tile, 32);
  if this.context == "Weapon" { tile = this.iconstm.get_tile(23, 1);} else { tile = this.iconstm.get_tile(22, 1); }
  this.cmd.draw_tile_sized(this.actions.Weapon.pos, tile, 32);
  if this.context == "Gear" { tile = this.iconstm.get_tile(23, 0);} else { tile = this.iconstm.get_tile(22, 0); }
  this.cmd.draw_tile_sized(this.actions.Gear.pos, tile, 32);
  if this.context == "Inventory" { tile = this.iconstm.get_tile(11, 0);} else { tile = this.iconstm.get_tile(10, 0); }
  this.cmd.draw_tile_sized(this.actions.Inventory.pos, tile, 32);

  // Player Info
  this.cmd.draw_text_rect(rect(671, 26, 326, 20), "" + this.player.name, this.font_name, 19.0, rgb(150, 150, 150), "left");
  this.cmd.draw_text_rect(rect(671, 26, 326, 20), "" + this.player.gold + "G", this.font_name, 19.0, rgb(150, 150, 150), "right");
  this.cmd.draw_text_rect(rect(671, 50, 326, 20), "HP: " + this.player.HP + "  STR:" + this.player.STR, this.font_name, 19.0, rgb(150, 150, 150), "left");

  // Messages
  this.cmd.draw_messages(this.board, this.font_name, 16.0, this.silver);

  if this.context == "Inventory" {
    let y = 90;
    let color = rgb(150, 150, 150);
    for item in 0..this.inventory.len() {
      if this.item_id == item && this.item_highlight > 0 {
        color = this.silver;
        this.item_highlight -= 1;
      }
      this.cmd.draw_text(pos(671, y), "" + (item + 1) + ". " + this.inventory.item_name_at(item), this.font_name, 19.0, color);
      this.cmd.draw_text_rect(rect(671, y, 326, 20), "" + this.inventory.item_amount_at(item), this.font_name, 19.0, color, "right");
      y += 20;
    }
  }

  // Info Text
  this.cmd.draw_text(pos(10, this.height - 22), "FOR MORE INFORMATION PLEASE VISIT ELDIRON.COM", this.font_name, 14.0, rgb(150, 150, 150));
}

// Handle the mouse events
fn touch_down(x, y) {
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

  this.handle_action(dir);

  for item in this.actions.keys() {
    if this.actions[item].is_inside(pos(x, y) ) {
      if item == "Messages" || item == "Gear" || item == "Inventory" || item == "Weapon" {
        this.context = item;
      } else {
        this.curr_action = item;
      }
    }
  }

  // Item based action ?
  if this.items.is_inside(pos(x, y)) {
    let item = (y - this.items.y) / 20;
    if this.inventory.len() > item {
      this.cmd.action_inventory(this.curr_action, item);
      this.item_id = 0;
      this.item_highlight = 2;
      this.curr_action = "Move";
    }
  }
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
  if key == "o" {
    this.context = "Messages";
  } else
  if key == "g" {
    this.context = "Gear";
  } else
  if key == "w" {
    this.context = "Weapon";
  }

  // Inventory Action ?
  if this.context == "Inventory" {
    if key == "1" {
      if this.inventory.len() > 0 {
        this.cmd.action_inventory(this.curr_action, 0);
        this.item_id = 0;
        this.item_highlight = 2;
        this.curr_action = "Move";
      }
    }
  }

  this.handle_action(dir);
}

// Execute action for the given direction
fn handle_action(dir) {
  if dir.len() > 0 {
    this.movement_id = dir;
    this.movement_visible = 2;
    this.cmd.action(this.curr_action, dir);
    this.curr_action = "Move";
  }
}