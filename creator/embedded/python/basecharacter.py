from array import array

class NewCharacter(Entity):
    def __init__(self):
        """Initializes the Player."""

        self.type = EntityType.PLAYER

    def init(self):
        """Init the entity."""
        pass

    def event(self, event, value):
        pass
        # print("Player Event", event, value)

    def user_event(self, event, value):
        # print("Player User Event", event, value)
        if event == 'key_down':
            if value == 'w':
                action("forward")
            if value == 'a':
                action("left")
            if value == 'd':
                action("right")
            if value == 's':
                action("backward")
        if event == 'key_up':
                action("none")

        # match event:
        #    case "a":
        #        action(self.id, self.manager_id, EntityAction.NORTH)
