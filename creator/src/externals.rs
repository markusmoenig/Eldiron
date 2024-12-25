use crate::editor::CODEEDITOR;
use crate::prelude::*;

pub fn set_server_externals() {
    let mut codeeditor = CODEEDITOR.lock().unwrap();

    codeeditor.clear_externals();

    codeeditor.add_external(TheExternalCode::new(
        "KeyDown".to_string(),
        "Returns the currently pressed key (if any).".to_string(),
        vec![],
        vec![],
        Some(TheValue::Text("".to_string())),
    ));

    codeeditor.add_external(TheExternalCode::new(
        "Region".to_string(),
        "Returns the selected region property.".to_string(),
        vec!["Property".to_string()],
        vec![TheValue::TextList(
            0,
            vec![
                "Property #1".to_string(),
                "Property #2".to_string(),
                "Property #3".to_string(),
                "Property #4".to_string(),
            ],
        )],
        Some(TheValue::Text("".to_string())),
    ));

    codeeditor.add_external(TheExternalCode::new(
        "Tell".to_string(),
        "Tells the current target the given text.".to_string(),
        vec![str!("Text")],
        vec![TheValue::Text("".to_string())],
        None,
    ));

    /*
    codeeditor.add_external(TheExternalCode::new(
        "RandWalk".to_string(),
        "Moves the character in a random direction.".to_string(),
        vec![],
        vec![],
        None,
    ));*/

    codeeditor.add_external(TheExternalCode::new(
        "Pulse".to_string(),
        "Counts up to the value in \"Count to\" and returns true on completion. Then restarts."
            .to_string(),
        vec!["Count to".to_string()],
        vec![TheValue::Int(4)],
        Some(TheValue::Bool(false)),
    ));

    codeeditor.add_external(TheExternalCode::new(
        "Move".to_string(),
        "Moves the character in the specified direction.".to_string(),
        vec!["By".to_string()],
        vec![TheValue::Float2(Vec2::new(0.0, 0.0))],
        Some(TheValue::Bool(false)),
    ));

    codeeditor.add_external(TheExternalCode::new(
        "Walk".to_string(),
        "Walks the character in the given direction relative to the direction he is facing."
            .to_string(),
        vec!["Direction".to_string(), "Distance".to_string()],
        vec![
            TheValue::TextList(
                0,
                vec![
                    "Forward".to_string(),
                    "Backward".to_string(),
                    "Left".to_string(),
                    "Right".to_string(),
                ],
            ),
            TheValue::Float(1.0),
        ],
        Some(TheValue::Bool(false)),
    ));

    codeeditor.add_external(TheExternalCode::new(
        "Rotate".to_string(),
        "Rotates the character in 2D by the specified angle.".to_string(),
        vec!["Angle".to_string()],
        vec![TheValue::Float(0.0)],
        None,
    ));

    codeeditor.add_external(TheExternalCode::new(
        "InArea".to_string(),
        "Returns the amount of characters in the area.".to_string(),
        vec![],
        vec![],
        Some(TheValue::Int(0)),
    ));

    codeeditor.add_external(TheExternalCode::new(
        "Create".to_string(),
        "Creates the item identified by its name.".to_string(),
        vec![str!("Item")],
        vec![TheValue::Text(str!("name"))],
        Some(TheValue::CodeObject(TheCodeObject::default())),
    ));

    codeeditor.add_external(TheExternalCode::new(
        "WallFX".to_string(),
        "Applies an effect on the wall at the given position.".to_string(),
        vec!["Position".to_string(), "FX".to_string()],
        vec![
            TheValue::Position(Vec3::new(0.0, 0.0, 0.0)),
            TheValue::TextList(
                0,
                vec![
                    "Normal".to_string(),
                    "Move Up".to_string(),
                    "Move Right".to_string(),
                    "Move Down".to_string(),
                    "Move Left".to_string(),
                    "Fade Out".to_string(),
                ],
            ),
        ],
        None,
    ));

    codeeditor.add_external(TheExternalCode::new(
        "Debug".to_string(),
        "Outputs the specified debug value.".to_string(),
        vec!["Value".to_string()],
        vec![TheValue::Text("Text".to_string())],
        None,
    ));
}

pub fn set_client_externals() {
    let mut codeeditor = CODEEDITOR.lock().unwrap();

    codeeditor.clear_externals();

    codeeditor.add_external(TheExternalCode::new(
        "DrawGame".to_string(),
        "Draws the game in the widget.".to_string(),
        vec![str!("Mode"), "Zoom".to_string()],
        vec![
            TheValue::TextList(0, vec!["2D".to_string(), "3D".to_string()]),
            TheValue::Float(1.0),
        ],
        None,
    ));

    codeeditor.add_external(TheExternalCode::new(
        "Fill".to_string(),
        "Fills the widget rectangle with the given color.".to_string(),
        vec![str!("Color")],
        vec![TheValue::ColorObject(TheColor::default())],
        None,
    ));

    codeeditor.add_external(TheExternalCode::new(
        "DrawText".to_string(),
        "Draws the given text.".to_string(),
        vec![str!("Font"), str!("Size"), str!("Text")],
        vec![
            TheValue::Text("font".to_string()),
            TheValue::Float(12.0),
            TheValue::Text("text".to_string()),
        ],
        None,
    ));

    codeeditor.add_external(TheExternalCode::new(
        "CreateImg".to_string(),
        "Creates an image from the given asset / tilemap name and position / size of the rectangle.".to_string(),
        vec![
            "Image".to_string(),
            "Position".to_string(),
            "Size".to_string(),
        ],
        vec![
            TheValue::Text("name".to_string()),
            TheValue::Int2(Vec2::new(0, 0)),
            TheValue::Int2(Vec2::new(100, 100)),
        ],
        Some(TheValue::Image(TheRGBABuffer::default())),
    ));

    codeeditor.add_external(TheExternalCode::new(
        "CreateTile".to_string(),
        "Creates a tile from the given tile tags and category.".to_string(),
        vec!["Category".to_string(), "Tags".to_string()],
        vec![
            TheValue::TextList(
                0,
                vec![
                    "All".to_string(),
                    "Character".to_string(),
                    "Nature".to_string(),
                    "Mountain".to_string(),
                    "Road".to_string(),
                    "Water".to_string(),
                    "Man Made".to_string(),
                    "Dungeon".to_string(),
                    "Effect".to_string(),
                    "Icon".to_string(),
                    "UI".to_string(),
                ],
            ),
            TheValue::Text("tags".to_string()),
        ],
        Some(TheValue::Image(TheRGBABuffer::default())),
    ));

    codeeditor.add_external(TheExternalCode::new(
        "DrawImg".to_string(),
        "Draws an image (or a tile) into the widget at the given position.".to_string(),
        vec!["Image".to_string(), "Position".to_string()],
        vec![
            TheValue::Image(TheRGBABuffer::default()),
            TheValue::Int2(Vec2::new(0, 0)),
        ],
        None,
    ));

    codeeditor.add_external(TheExternalCode::new(
        "Player".to_string(),
        "Instantiates the player of the given name with the new name.".to_string(),
        vec!["Name".to_string(), "As".to_string()],
        vec![TheValue::Text(str!("")), TheValue::Text(str!(""))],
        Some(TheValue::Image(TheRGBABuffer::default())),
    ));

    codeeditor.add_external(TheExternalCode::new(
        "ScaleImg".to_string(),
        "Scales an image and returns it.".to_string(),
        vec!["Image".to_string(), "Size".to_string()],
        vec![
            TheValue::Image(TheRGBABuffer::default()),
            TheValue::Int2(Vec2::new(0, 0)),
        ],
        Some(TheValue::Image(TheRGBABuffer::default())),
    ));

    codeeditor.add_external(TheExternalCode::new(
        "SendCmd".to_string(),
        "Sends the given command string to the server.".to_string(),
        vec!["Execute".to_string()],
        vec![TheValue::Text("".to_string())],
        None,
    ));

    codeeditor.add_external(TheExternalCode::new(
        "Start".to_string(),
        "Starts the server, only valid for solo games.".to_string(),
        vec![],
        vec![],
        None,
    ));
}
