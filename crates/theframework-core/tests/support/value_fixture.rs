use crate::fixture_api::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Fixture {
    pub values: Vec<TheValue>,
    pub tile: TheRGBATile,
    pub regions: TheRGBARegionSequence,
    pub descriptions: Vec<String>,
}
pub fn fixture() -> Fixture {
    let id = Uuid::from_u128(0x1234567890abcdef1122334455667788);
    let mut bitmap = TheRGBABuffer::new(TheDim::new(-3, 7, 3, 2));
    bitmap.set_render_scale(1.5);
    for (i, p) in bitmap.pixels_mut().iter_mut().enumerate() {
        *p = (i * 37 % 256) as u8;
    }
    let mut unit = TheRGBABuffer::new(TheDim::new(1, -2, 2, 1));
    unit.pixels_mut()
        .copy_from_slice(&[3, 5, 7, 0, 11, 13, 17, 255]);
    let mut mask = TheTileMask::new();
    mask.add_pixel(Vec2::new(-2, 3), true);
    mask.add_pixel(Vec2::new(7, -11), false);
    let begin = TheTime {
        hours: 8,
        minutes: 30,
        seconds: 45,
    };
    let end = TheTime {
        hours: 23,
        minutes: 59,
        seconds: 59,
    };
    let mut values = vec![
        TheValue::Empty,
        TheValue::Bool(true),
        TheValue::Float(-1.25),
        TheValue::FloatRange(1.5, -2.5..=8.125),
        TheValue::Int(-11),
        TheValue::IntRange(7, -9..=11),
        TheValue::Text("مرحبا 東京\r\nÅß".into()),
        TheValue::TextList(1, vec!["zero".into(), "one".into()]),
        TheValue::Char('ß'),
        TheValue::Int2(Vec2::new(-1, 2)),
        TheValue::Float2(Vec2::new(-1.25, 2.5)),
        TheValue::Int3(Vec3::new(1, -2, 3)),
        TheValue::Float3(Vec3::new(1.25, -2.5, 3.75)),
        TheValue::Int4(Vec4::new(1, 2, -3, 4)),
        TheValue::Float4(Vec4::new(1.25, 2.5, -3.75, 4.125)),
        TheValue::Position(Vec3::new(-1.25, 0., 3.5)),
        TheValue::Tile("sprite".into(), id),
        TheValue::RangeI32(-7..=19),
        TheValue::RangeF32(-1.5..=12.75),
        TheValue::ColorObject(TheColor {
            r: 0.25,
            g: 0.5,
            b: 0.75,
            a: 0.125,
            name: "ink".into(),
        }),
        TheValue::PaletteIndex(65535),
        TheValue::Id(id),
        TheValue::Direction(Vec3::new(0., -1., 0.)),
        TheValue::List(vec![TheValue::Bool(false), TheValue::Text("nested".into())]),
        TheValue::Time(begin),
        TheValue::TimeDuration(begin, end),
        TheValue::TileMask(mask),
        TheValue::Image(bitmap.clone()),
        TheValue::Image(unit.clone()),
    ];
    values.extend(
        [
            TheKeyCode::Escape,
            TheKeyCode::Return,
            TheKeyCode::Delete,
            TheKeyCode::Home,
            TheKeyCode::End,
            TheKeyCode::Up,
            TheKeyCode::Right,
            TheKeyCode::Down,
            TheKeyCode::Left,
            TheKeyCode::Space,
            TheKeyCode::Tab,
        ]
        .into_iter()
        .map(TheValue::KeyCode),
    );
    values.extend(TheValueComparison::iterator().map(TheValue::Comparison));
    values.extend(TheValueAssignment::iterator().map(TheValue::Assignment));
    let descriptions = values.iter().map(TheValue::describe).collect();
    Fixture {
        values,
        descriptions,
        tile: TheRGBATile {
            id,
            name: "atlas".into(),
            buffer: vec![bitmap, unit],
            role: 2,
            render_mode: 3,
            blocking: true,
            scale: 1.25,
        },
        regions: TheRGBARegionSequence {
            regions: vec![
                TheRGBARegion::new(1, 2, 3, 4),
                TheRGBARegion::new(0, 0, 2, 1),
            ],
        },
    }
}
