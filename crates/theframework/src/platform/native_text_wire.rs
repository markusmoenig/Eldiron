//! Shared UTF-16 native responder protocol. Source identity/revision must be
//! checked by the application before any range is interpreted.
use super::NativeTextAction;
use crate::text_edit::*;
use serde_json::{Value, json};

pub fn snapshot(source: Option<&NativeTextSnapshot>) -> Result<Value, &'static str> {
    let Some(source) = source else {
        return Ok(Value::Null);
    };
    let position = |index| {
        source
            .utf16_from_byte(index)
            .ok_or("Invalid native text position")
    };
    let selection = [position(source.selection.0)?, position(source.selection.1)?];
    let marked = source
        .composition
        .as_ref()
        .map(|r| Ok::<_, &'static str>([position(r.start)?, position(r.end)?]))
        .transpose()?;
    Ok(
        json!({"target":source.target.raw(),"revision":source.revision,"text":source.text.as_ref(),"selection":selection,"marked":marked}),
    )
}

pub fn query(
    source: &NativeTextSnapshot,
    kind: u32,
    start: isize,
    end: isize,
    x: f32,
    y: f32,
) -> Result<NativeTextRequest, &'static str> {
    let position = |index: isize| {
        usize::try_from(index)
            .ok()
            .and_then(|i| source.byte_from_utf16(i))
            .ok_or("Invalid native UTF-16 position")
    };
    let query = match kind {
        0 => NativeTextQuery::Geometry(
            source
                .byte_range_from_utf16(
                    usize::try_from(start).map_err(|_| "Invalid native text range")?,
                    usize::try_from(end).map_err(|_| "Invalid native text range")?,
                )
                .ok_or("Invalid native UTF-16 range")?,
        ),
        1 if x.is_finite() && y.is_finite() => NativeTextQuery::Hit([x, y]),
        2..=5 => NativeTextQuery::Navigate {
            position: position(start)?,
            direction: match kind {
                2 => NativeTextDirection::Left,
                3 => NativeTextDirection::Right,
                4 => NativeTextDirection::Up,
                _ => NativeTextDirection::Down,
            },
            count: usize::try_from(end).map_err(|_| "Invalid native movement count")?,
        },
        _ => return Err("Invalid native text query"),
    };
    Ok(NativeTextRequest {
        target: source.target,
        revision: source.revision,
        query,
    })
}

pub fn answer(source: &NativeTextSnapshot, answer: Option<NativeTextAnswer>) -> Value {
    match answer {
        Some(NativeTextAnswer::Geometry {
            caret,
            selection,
            first_range,
        }) => {
            let rect = |r: crate::paint::Rect| [r.x, r.y, r.width, r.height];
            json!({"caret":rect(caret),"selection":selection.into_iter().map(rect).collect::<Vec<_>>(),"first_range":[source.utf16_from_byte(first_range.start),source.utf16_from_byte(first_range.end)]})
        }
        Some(NativeTextAnswer::Hit(index)) => json!({"position":source.utf16_from_byte(index)}),
        None => Value::Null,
    }
}

pub fn edit(
    source: &NativeTextSnapshot,
    kind: u32,
    start: isize,
    end: isize,
    text: String,
    cursor_start: isize,
    cursor_end: isize,
) -> Result<NativeTextAction, &'static str> {
    let position = |index: isize| {
        usize::try_from(index)
            .ok()
            .and_then(|i| source.byte_from_utf16(i))
            .ok_or("Native range must use valid UTF-16 positions")
    };
    let range = || {
        let (start, end) = (position(start)?, position(end)?);
        if start > end {
            return Err("Native replacement range is reversed");
        }
        Ok(start..end)
    };
    Ok(match kind {
        0 => NativeTextAction::Select {
            anchor: position(start)?,
            focus: position(end)?,
        },
        1 => NativeTextAction::Replace {
            range: range()?,
            text,
        },
        2 => {
            let cursor = if cursor_start == -1 && cursor_end == -1 {
                None
            } else {
                let index = |i: isize| {
                    usize::try_from(i)
                        .ok()
                        .and_then(|i| utf16_byte_offset(&text, i))
                        .ok_or("Native preedit cursor must use valid UTF-16 positions")
                };
                Some((index(cursor_start)?, index(cursor_end)?))
            };
            NativeTextAction::Compose {
                range: if start == -1 && end == -1 {
                    None
                } else {
                    Some(range()?)
                },
                text,
                cursor,
            }
        }
        3 => NativeTextAction::CommitComposition,
        4 => NativeTextAction::CancelComposition,
        5 => NativeTextAction::Insert {
            selection: (position(start)?, position(end)?),
            text,
        },
        6 => NativeTextAction::DeleteBackward {
            selection: (position(start)?, position(end)?),
        },
        _ => return Err("Unknown native text operation"),
    })
}
