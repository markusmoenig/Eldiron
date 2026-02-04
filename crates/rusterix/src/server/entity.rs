use indexmap::IndexMap;
use rand::Rng;
use theframework::prelude::*;
use vek::{Vec2, Vec3};

use crate::{EntityAction, prelude::*};

/// The Rust representation of an Entity. The real entity class lives in Python, this class is the Rust side
/// instantiation (to avoid unnecessary Python look ups for common attributes). The class gets synced with the Python side.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entity {
    /// The id of the entity in the entity manager
    pub id: u32,

    /// Maps the entity to a creator id
    pub creator_id: Uuid,

    /// The XZ orientation
    pub orientation: Vec2<f32>,
    /// The position in the map
    pub position: Vec3<f32>,
    /// The vertical camera tilt, 0.0 means flat, no tilt.
    pub tilt: f32,

    /// The current action, server side.
    #[serde(skip)]
    pub action: EntityAction,

    /// Attributes
    pub attributes: ValueContainer,

    /// Dirty static attributes
    /// The `dirty_flags` field is a bitmask representing changes to various components of the entity.
    /// Each bit corresponds to a specific type of change:
    /// - `0b00000001` (1): Position changed
    /// - `0b00000010` (2): Orientation changed
    /// - `0b00000100` (4): Tilt changed
    /// - `0b00001000` (8): Inventory changed
    /// - `0b00010000` (16): Equipped items changed
    /// - `0b00100000` (32): Wallet changed
    pub dirty_flags: u8,

    /// Dirty Attributes
    pub dirty_attributes: FxHashSet<String>,

    /// Inventory: A container for the entity's items
    #[serde(skip_deserializing)]
    pub inventory: Vec<Option<Item>>,

    /// Track added items
    pub inventory_additions: FxHashMap<usize, Item>,
    /// Track removed items
    pub inventory_removals: FxHashSet<usize>,
    /// Track updated items
    pub inventory_updates: FxHashMap<usize, ItemUpdate>,

    /// Equipped items
    pub equipped: IndexMap<String, Item>,

    /// Wallet
    pub wallet: Wallet,
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity {
    pub fn new() -> Self {
        Self {
            id: 0,
            creator_id: Uuid::new_v4(),

            orientation: Vec2::new(1.0, 0.0),
            position: Vec3::new(0.0, 1.0, 0.0),
            tilt: 0.0,

            action: EntityAction::Off,

            attributes: ValueContainer::default(),

            dirty_flags: 0,
            dirty_attributes: FxHashSet::default(),

            inventory: vec![],
            inventory_additions: FxHashMap::default(),
            inventory_removals: FxHashSet::default(),
            inventory_updates: FxHashMap::default(),

            equipped: IndexMap::default(),

            wallet: Wallet::default(),
        }
    }

    /// Get the entity mode.
    pub fn get_mode(&self) -> String {
        self.attributes.get_str_default("mode", "active".into())
    }

    /// Get the XZ position.
    pub fn get_pos_xz(&self) -> Vec2<f32> {
        Vec2::new(self.position.x, self.position.z)
    }

    /// Computes the look-at target based on position, orientation, and vertical tilt (tilt).
    pub fn camera_look_at(&self) -> Vec3<f32> {
        let vertical_offset = self.orientation.magnitude() * self.tilt.sin();
        Vec3::new(
            self.position.x + self.orientation.x,
            self.position.y + vertical_offset,
            self.position.z + self.orientation.y,
        )
    }

    /// A forward direction vector for the entity.
    pub fn forward(&self) -> Vec3<f32> {
        let dir_xz = self.orientation.normalized();
        let cos_tilt = self.tilt.cos();
        let sin_tilt = self.tilt.sin();

        Vec3::new(dir_xz.x * cos_tilt, sin_tilt, dir_xz.y * cos_tilt).normalized()
    }

    /// Rotates the entity to the left by a certain degree.
    pub fn turn_left(&mut self, degrees: f32) {
        self.rotate_orientation(-degrees.to_radians());
    }

    /// Rotates the entity to the right by a certain degree.
    pub fn turn_right(&mut self, degrees: f32) {
        self.rotate_orientation(degrees.to_radians());
    }

    /// Moves the entity forward along its current orientation.
    pub fn move_forward(&mut self, distance: f32) {
        self.position.x += self.orientation.x * distance;
        self.position.z += self.orientation.y * distance;
        self.mark_dirty_field(0b0001);
    }

    /// Moves the entity backward along its current orientation.
    pub fn move_backward(&mut self, distance: f32) {
        self.position.x -= self.orientation.x * distance;
        self.position.z -= self.orientation.y * distance;
        self.mark_dirty_field(0b0001);
    }

    /// Gets the position in the direction of the entity's orientation by a given distance.
    pub fn get_forward_pos(&mut self, distance: f32) -> Vec2<f32> {
        Vec2::new(
            self.position.x + self.orientation.x * distance,
            self.position.z + self.orientation.y * distance,
        )
    }

    /// Helper method to rotate the orientation vector by a given angle in radians.
    fn rotate_orientation(&mut self, radians: f32) {
        let cos_angle = radians.cos();
        let sin_angle = radians.sin();
        let new_x = self.orientation.x * cos_angle - self.orientation.y * sin_angle;
        let new_y = self.orientation.x * sin_angle + self.orientation.y * cos_angle;
        self.orientation = Vec2::new(new_x, new_y).normalized();
        self.mark_dirty_field(0b0010);
    }

    /// Applies the camera's position and look-at parameters based on the entity's state.
    pub fn apply_to_camera(&self, camera: &mut Box<dyn D3Camera>) {
        // println!("{} {}", self.position, self.orientation);
        let id = camera.id();

        if id != "iso" {
            camera.set_parameter_vec3("position", self.position);
            camera.set_parameter_vec3("center", self.camera_look_at());
        } else {
            let p = Vec3::new(self.position.x, 0.0, self.position.z);
            camera.set_parameter_vec3("center", p);
            camera.set_parameter_vec3("position", p + vek::Vec3::new(-10.0, 10.0, 10.0));
        }
    }

    /// Set the position and mark it as dirty
    pub fn set_position(&mut self, new_position: Vec3<f32>) {
        if self.position != new_position {
            self.position = new_position;
            self.mark_dirty_field(0b0001);
        }
    }

    /// Set the position as a Vec2 and mark it as dirty
    pub fn set_pos_xz(&mut self, new_position: Vec2<f32>) {
        self.position.x = new_position.x;
        self.position.z = new_position.y;
        self.mark_dirty_field(0b0001);
    }

    /// Set the orientation and mark it as dirty
    pub fn set_orientation(&mut self, new_orientation: Vec2<f32>) {
        if self.orientation != new_orientation {
            self.orientation = new_orientation;
            self.mark_dirty_field(0b0010);
        }
    }

    /// Set the tilt and mark it as dirty
    pub fn set_tilt(&mut self, new_tilt: f32) {
        if self.tilt != new_tilt {
            self.tilt = new_tilt;
            self.mark_dirty_field(0b0100);
        }
    }

    /// Maps a normalized screen coordinate (0.0 to 1.0) to a `tilt` angle.
    /// `0.0` -> maximum downward tilt, `1.0` -> maximum upward tilt.
    pub fn set_tilt_from_screen_coordinate(&mut self, screen_y: f32) {
        // Map the normalized screen coordinate to a range of angles (e.g., -π/4 to π/4)
        let max_tilt = std::f32::consts::FRAC_PI_4; // 45 degrees
        self.tilt = (screen_y - 0.5) * 2.0 * max_tilt;
        self.mark_dirty_field(0b0100);
    }

    /// Add an item to the entity's inventory and track additions
    pub fn add_item(&mut self, item: Item) -> Result<usize, String> {
        if let Some(slot) = self.inventory.iter_mut().position(|i| i.is_none()) {
            self.inventory[slot] = Some(item.clone());
            self.inventory_additions.insert(slot, item);
            self.inventory_removals.remove(&slot);
            self.mark_dirty_field(0b1000);
            Ok(slot)
        } else {
            Err("Inventory full".into())
        }
    }

    /// Remove an item from the given slot.
    pub fn remove_item_from_slot(&mut self, slot: usize) -> Option<Item> {
        if slot < self.inventory.len() {
            if let Some(item) = self.inventory[slot].take() {
                self.inventory_removals.insert(slot);
                self.inventory_additions.remove(&slot);
                self.mark_dirty_field(0b1000);
                return Some(item);
            }
        }
        None
    }

    /// Remove an item from the entity's inventory and track removals
    pub fn remove_item(&mut self, id: u32) -> Option<Item> {
        for (slot, opt_item) in self.inventory.iter_mut().enumerate() {
            if let Some(item) = opt_item {
                if item.id == id {
                    return self.remove_item_from_slot(slot);
                }
            }
        }
        None
    }

    /// Get a reference to an item in a given slot.
    pub fn get_item_in_slot(&self, slot: usize) -> Option<&Item> {
        self.inventory.get(slot)?.as_ref()
    }

    /// Get a mutable reference to an item in a given slot.
    pub fn get_item_in_slot_mut(&mut self, slot: usize) -> Option<&mut Item> {
        if let Some(Some(item)) = self.inventory.get_mut(slot) {
            self.inventory_updates.insert(slot, item.get_update());
            Some(item)
        } else {
            None
        }
    }

    /// Get a reference to an item by its ID
    pub fn get_item(&self, item_id: u32) -> Option<&Item> {
        self.inventory
            .iter()
            .filter_map(|opt_item| opt_item.as_ref())
            .find(|item| item.id == item_id)
    }

    /// Get a mutable reference to an item by its ID
    pub fn get_item_mut(&mut self, item_id: u32) -> Option<&mut Item> {
        for (slot, opt_item) in self.inventory.iter_mut().enumerate() {
            if let Some(item) = opt_item {
                if item.id == item_id {
                    self.inventory_updates.insert(slot, item.get_update());
                    return Some(item);
                }
            }
        }
        None
    }

    /// Get the slot index of an item by its ID
    pub fn get_item_slot(&self, item_id: u32) -> Option<usize> {
        self.inventory.iter().position(|opt_item| {
            opt_item
                .as_ref()
                .map(|item| item.id == item_id)
                .unwrap_or(false)
        })
    }

    /// Equip an item into a specific slot
    pub fn equip_item(&mut self, item_id: u32, slot: &str) -> Result<(), String> {
        if let Some(item) = self.remove_item(item_id) {
            if let Some(old_item) = self.equipped.shift_remove(slot) {
                _ = self.add_item(old_item);
            }
            self.equipped.insert(slot.to_string(), item);
            self.dirty_flags |= 0b10000;
            Ok(())
        } else {
            Err("Item not found in inventory.".to_string())
        }
    }

    /// Unequip an item from a specific slot
    pub fn unequip_item(&mut self, slot: &str) -> Result<Item, String> {
        if let Some(item) = self.equipped.shift_remove(slot) {
            self.dirty_flags |= 0b10000; // Mark equipped slots as dirty
            Ok(item)
        } else {
            Err("No item equipped in the given slot.".to_string())
        }
    }

    /// Get a reference of an item equipped in a specific slot
    pub fn get_equipped_item(&self, slot: &str) -> Option<&Item> {
        self.equipped.get(slot)
    }

    /// Get the mutable reference of an item equipped in a specific slot
    pub fn get_equipped_item_mut(&mut self, slot: &str) -> Option<&mut Item> {
        self.equipped.get_mut(slot)
    }

    /// Add the given currency to the wallet.
    pub fn add_currency(
        &mut self,
        symbol: &str,
        amount: i64,
        currencies: &Currencies,
    ) -> Result<(), String> {
        self.wallet.add(symbol, amount, currencies)?;
        self.dirty_flags |= 0b100000;
        Ok(())
    }

    /// Add the given base currency to the wallet.
    pub fn add_base_currency(
        &mut self,
        amount: i64,
        currencies: &Currencies,
    ) -> Result<(), String> {
        self.wallet.add_base_currency(amount, currencies)?;
        self.dirty_flags |= 0b100000;
        Ok(())
    }

    /// Spend the given currency.
    pub fn spend_currency(
        &mut self,
        base_amount: i64,
        currencies: &Currencies,
    ) -> Result<(), String> {
        self.wallet.spend(base_amount, currencies)?;
        self.dirty_flags |= 0b100000;
        Ok(())
    }

    /// Set a dynamic attribute and mark it as dirty
    pub fn set_attribute(&mut self, key: &str, value: Value) {
        self.attributes.set(key, value);
        self.mark_dirty_attribute(key);
    }

    /// Get the given String
    pub fn get_attr_string(&self, key: &str) -> Option<String> {
        self.attributes.get(key).map(|value| value.to_string())
    }

    /// Get the given Uuid
    pub fn get_attr_uuid(&self, key: &str) -> Option<Uuid> {
        if let Some(Value::Id(value)) = self.attributes.get(key) {
            Some(*value)
        } else {
            None
        }
    }

    /// Returns true if this entity is a player
    pub fn is_player(&self) -> bool {
        if let Some(Value::Bool(value)) = self.attributes.get("player") {
            *value
        } else {
            false
        }
    }

    /// Mark a static field as dirty
    fn mark_dirty_field(&mut self, field: u8) {
        self.dirty_flags |= field;
    }

    /// Mark a dynamic attribute as dirty
    pub fn mark_dirty_attribute(&mut self, key: &str) {
        self.dirty_attributes.insert(key.to_string());
    }

    /// Mark all fields and attributes as dirty.
    pub fn mark_all_dirty(&mut self) {
        self.dirty_flags = 0b11111;
        self.dirty_attributes = self.attributes.keys().cloned().collect();
    }

    /// Check if the entity is dirty and generate item updates as needed.
    pub fn is_dirty(&mut self) -> bool {
        let mut is_dirty = self.dirty_flags != 0 || !self.dirty_attributes.is_empty();

        for (slot, item) in self.inventory.iter_mut().enumerate() {
            if let Some(item) = item {
                if item.is_dirty() {
                    self.inventory_updates.insert(slot, item.get_update());
                    is_dirty = true;
                }
            }
        }

        is_dirty
    }

    /// Mark all static fields as dirty
    pub fn set_static_dirty(&mut self) {
        self.dirty_flags = 0b11111;
        self.dirty_attributes.clear();
    }

    /// Clear all dirty flags and attributes
    pub fn clear_dirty(&mut self) {
        self.dirty_flags = 0;
        self.dirty_attributes.clear();
        self.inventory_additions.clear();
        self.inventory_removals.clear();
        self.inventory_updates.clear();
        for item in self.inventory.iter_mut() {
            if let Some(item) = item {
                item.clear_dirty();
            }
        }
    }

    /// Get a partial update containing only dirty fields and attributes
    pub fn get_update(&self) -> EntityUpdate {
        EntityUpdate {
            id: self.id,
            creator_id: self.creator_id,
            position: if self.dirty_flags & 0b0001 != 0 {
                Some(self.position)
            } else {
                None
            },
            orientation: if self.dirty_flags & 0b0010 != 0 {
                Some(self.orientation)
            } else {
                None
            },
            tilt: if self.dirty_flags & 0b0100 != 0 {
                Some(self.tilt)
            } else {
                None
            },
            attributes: self
                .dirty_attributes
                .iter()
                .filter_map(|key| self.attributes.get(key).map(|v| (key.clone(), v.clone())))
                .collect(),
            inventory_additions: if !self.inventory_additions.is_empty() {
                Some(self.inventory_additions.clone())
            } else {
                None
            },
            inventory_removals: if !self.inventory_removals.is_empty() {
                Some(self.inventory_removals.clone())
            } else {
                None
            },
            inventory_updates: if !self.inventory_updates.is_empty() {
                Some(self.inventory_updates.clone())
            } else {
                None
            },
            equipped_updates: if self.dirty_flags & 0b10000 != 0 {
                Some(self.equipped.clone())
            } else {
                None
            },
            wallet_updates: if self.dirty_flags & 0b100000 != 0 {
                Some(self.wallet.balances.clone())
            } else {
                None
            },
        }
    }

    /// Apply an update to the entity. Returns true if the entities appearance has changed and needs to be updated.
    pub fn apply_update(&mut self, update: EntityUpdate) -> bool {
        // Validate ID matches
        if self.id != update.id {
            eprintln!("Update ID does not match Entity ID!");
            return false;
        }

        let mut rc = false;

        self.creator_id = update.creator_id;

        // Update static fields
        if let Some(new_position) = update.position {
            self.position = new_position;
        }
        if let Some(new_orientation) = update.orientation {
            self.orientation = new_orientation;
        }
        if let Some(new_camera_tilt) = update.tilt {
            self.tilt = new_camera_tilt;
        }

        // Update dynamic attributes
        for (key, value) in update.attributes {
            self.attributes.set(&key, value.clone());
            self.mark_dirty_attribute(&key);
        }

        if let Some(inventory_additions) = update.inventory_additions {
            let required_len = inventory_additions.keys().copied().max().unwrap_or(0) + 1;
            if self.inventory.len() < required_len {
                self.inventory.resize(required_len, None);
            }

            for (slot, item) in inventory_additions {
                self.inventory[slot] = Some(item);
            }
        }

        if let Some(inventory_removals) = update.inventory_removals {
            for slot in inventory_removals {
                if slot < self.inventory.len() {
                    self.inventory[slot] = None;
                }
            }
        }

        if let Some(inventory_updates) = update.inventory_updates {
            for (slot, update) in inventory_updates {
                if let Some(Some(item)) = self.inventory.get_mut(slot) {
                    item.apply_update(update);
                }
            }
        }

        // Apply equipped slot updates
        if let Some(equipped_updates) = update.equipped_updates {
            rc = true;
            self.equipped = equipped_updates;
        }

        // Apply wallet updates
        if let Some(wallet_updates) = update.wallet_updates {
            for (symbol, balance) in wallet_updates {
                self.wallet.balances.insert(symbol, balance);
            }
        }

        rc
    }

    /// Sets the orientation to face east.
    pub fn face_east(&mut self) {
        self.set_orientation(Vec2::new(1.0, 0.0));
    }

    /// Sets the orientation to face west.
    pub fn face_west(&mut self) {
        self.set_orientation(Vec2::new(-1.0, 0.0));
    }

    /// Sets the orientation to face north.
    pub fn face_north(&mut self) {
        self.set_orientation(Vec2::new(0.0, -1.0));
    }

    /// Sets the orientation to face south.
    pub fn face_south(&mut self) {
        self.set_orientation(Vec2::new(0.0, 1.0));
    }

    /// Sets the orientation to face a specific point.
    pub fn face_at(&mut self, target: Vec2<f32>) {
        let current_position = self.get_pos_xz();
        let delta = target - current_position;
        if delta.magnitude_squared() < f32::EPSILON {
            return; // Don't face if target is the same as current
        }
        let direction = delta.normalized();
        self.set_orientation(direction);
    }

    /// Sets the orientation to face a random direction.
    pub fn face_random(&mut self) {
        let mut rng = rand::rng();
        let angle = rng.random_range(0.0..std::f32::consts::TAU); // TAU is 2π
        let direction = Vec2::new(angle.cos(), angle.sin());
        self.set_orientation(direction);
    }

    /// Create an iterator over the inventory.
    pub fn iter_inventory(&self) -> impl Iterator<Item = (usize, &Item)> {
        self.inventory
            .iter()
            .enumerate()
            .filter_map(|(slot, item)| item.as_ref().map(|i| (slot, i)))
    }

    /// Create a mutable iterator over the inventory.
    pub fn iter_inventory_mut(&mut self) -> impl Iterator<Item = (usize, &mut Item)> {
        self.inventory
            .iter_mut()
            .enumerate()
            .filter_map(|(slot, item)| {
                item.as_mut().map(|i| {
                    self.inventory_updates.insert(slot, i.get_update());
                    (slot, i)
                })
            })
    }
}

// EntityUpdate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityUpdate {
    pub id: u32,
    pub creator_id: Uuid,
    pub position: Option<Vec3<f32>>,
    pub orientation: Option<Vec2<f32>>,
    pub tilt: Option<f32>,
    pub attributes: FxHashMap<String, Value>,
    pub inventory_additions: Option<FxHashMap<usize, Item>>,
    pub inventory_removals: Option<FxHashSet<usize>>,
    pub inventory_updates: Option<FxHashMap<usize, ItemUpdate>>,
    pub equipped_updates: Option<IndexMap<String, Item>>,
    pub wallet_updates: Option<FxHashMap<String, i64>>,
}

impl EntityUpdate {
    /// Serialize (pack) an `EntityUpdate` into a `Vec<u8>` using bincode, discarding errors
    pub fn pack(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_else(|_| Vec::new())
    }

    /// Deserialize (unpack) a `Vec<u8>` into an `EntityUpdate` using bincode, discarding errors
    pub fn unpack(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap_or_else(|_| Self {
            id: 0,
            creator_id: Uuid::nil(),
            position: None,
            orientation: None,
            tilt: None,
            attributes: FxHashMap::default(),
            inventory_updates: None,
            inventory_additions: None,
            inventory_removals: None,
            equipped_updates: None,
            wallet_updates: None,
        })
    }
}
