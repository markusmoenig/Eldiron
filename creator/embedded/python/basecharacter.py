from array import array

class NewCharacter(Entity):
    def __init__(self):
        """Initializes the Player."""
        super().__init__()

        self.type = EntityType.PLAYER

    def init(self):
        """Init the entity."""
        pass

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
