class NewCharacter:

    def event(self, event, value):
        print("Player Event", event, value)

    def user_event(self, event, value):
        print("Player User Event", event, value)
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
