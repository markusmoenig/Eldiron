use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum TheDropOperation {
    Copy,
    Move,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TheDrop {
    pub id: TheId,
    pub target_id: TheId,

    pub data: String,
    pub title: String,
    pub text: String,
    pub image: TheRGBABuffer,

    pub operation: TheDropOperation,

    pub start_position: Option<Vec2<i32>>,
    pub position: Option<Vec2<i32>>,
    pub offset: Vec2<i32>,
}

impl TheDrop {
    pub fn new(id: TheId) -> Self {
        Self {
            id,
            target_id: TheId::empty(),

            data: String::new(),
            title: String::new(),
            text: String::new(),
            image: TheRGBABuffer::empty(),
            operation: TheDropOperation::Move,
            start_position: None,
            position: None,
            offset: Vec2::zero(),
        }
    }

    pub fn set_position(&mut self, position: Vec2<i32>) {
        self.position = Some(position);
    }

    pub fn set_offset(&mut self, offset: Vec2<i32>) {
        self.offset = offset;
    }

    pub fn set_data(&mut self, json: String) {
        self.data = json;
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub fn set_text(&mut self, title: String) {
        self.text = title;
    }

    pub fn set_image(&mut self, image: TheRGBABuffer) {
        self.image = image;
    }
}
