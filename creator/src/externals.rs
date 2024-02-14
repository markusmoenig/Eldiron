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
        vec![TheValue::Float2(vec2f(0.0, 0.0))],
        Some(TheValue::Bool(false)),
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
            TheValue::Position(vec3f(0.0, 0.0, 0.0)),
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
        "DrGame".to_string(),
        "Draws the game in the widget.".to_string(),
        vec!["Zoom".to_string()],
        vec![TheValue::Float(1.0)],
        None,
    ));

    codeeditor.add_external(TheExternalCode::new(
        "Fill".to_string(),
        "Fills the widget rectangle with the given color.".to_string(),
        vec![str!("Color")],
        vec![TheValue::ColorObject(TheColor::default(), 0.0)],
        None,
    ));

    codeeditor.add_external(TheExternalCode::new(
        "DrText".to_string(),
        "Draws the given text.".to_string(),
        vec![str!("Font"), str!("Size"), str!("Text")],
        vec![TheValue::Text("font".to_string()), TheValue::Float(12.0), TheValue::Text("text".to_string())],
        None,
    ));
}
