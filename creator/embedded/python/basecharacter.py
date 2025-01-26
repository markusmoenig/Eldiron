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
                action(self.id, "forward")
            if value == 'a':
                action(self.id, "left")
            if value == 'd':
                action(self.id, "right")
            if value == 's':
                action(self.id, "backward")
        if event == 'key_up':
                action(self.id, "none")

        # match event:
        #    case "a":
        #        action(self.id, self.manager_id, EntityAction.NORTH)
