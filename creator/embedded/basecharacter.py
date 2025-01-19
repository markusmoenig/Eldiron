from array import array

class Character(Entity):
    def __init__(self, position=None, orientation=None, attributes=None, level=1):
        """Initializes the Player."""
        super().__init__(position, orientation, attributes)

        self.type = EntityType.PLAYER

    def attack(self, target):
        """Attacks another fighter, reducing its health."""
        if not isinstance(target, Monster):
            print("Target must be an EntityFighter.")
            return

        target.take_damage(self.damage)
        print(f"{self} attacked {target}, dealing {self.damage} damage!")

    def take_damage(self, amount):
        """Reduces the fighter's health."""
        self.health -= amount
        print(f"{self} took {amount} damage, health is now {self.health}")
        if self.health <= 0:
            print(f"{self} has been defeated!")

    def update(self):
        """Update"""
        pass

    def event(self, event, value):
        pass
        # print("Player Event", event, value)

    def user_event(self, event, value):
        # print("Player User Event", event, value)
        if event == 'key_down':
            if value == 'w':
                action(self.id, EntityAction.NORTH.to_int())
            if value == 'a':
                action(self.id, EntityAction.WEST.to_int())
            if value == 'd':
                action(self.id, EntityAction.EAST.to_int())
            if value == 's':
                action(self.id, EntityAction.SOUTH.to_int())
        if event == 'key_up':
                action(self.id, EntityAction.NONE.to_int())

        # match event:
        #    case "a":
        #        action(self.id, self.manager_id, EntityAction.NORTH)


    def __str__(self):
        """String representation of the fighter."""
        return f"EntityFighter at {list(self.position)} with {self.health} HP and {self.damage} damage"
