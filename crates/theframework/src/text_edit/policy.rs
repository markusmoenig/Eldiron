//! Renderer-independent structural code edits, expressed as UTF-8 byte plans.
//! The controller applies each plan as one undo transaction on its shaped text.
use std::ops::Range;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditorTab {
    #[default]
    Focus,
    Spaces(usize),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditorUndo {
    #[default]
    Internal,
    /// A document/application transaction owner handles Undo/Redo shortcuts.
    External,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditorPolicy {
    pub read_only: bool,
    /// Form-style insertion/Enter submission while preserving exact externally
    /// loaded multiline source. Typed/pasted control characters are filtered;
    /// IME composition still owns Enter until it is committed.
    pub single_line_input: bool,
    pub undo: EditorUndo,
    pub tab: EditorTab,
    pub auto_indent: bool,
    pub auto_brackets: bool,
}
pub(super) struct Edit {
    pub range: Range<usize>,
    pub text: String,
}
pub(super) struct EditPlan {
    /// Sorted in ascending source order; application proceeds in reverse.
    pub edits: Vec<Edit>,
    pub selection: (usize, usize),
}
fn line_start(source: &str, index: usize) -> usize {
    source[..index].rfind('\n').map_or(0, |pos| pos + 1)
}
pub(super) fn newline(source: &str, selection: (usize, usize), indent: bool) -> EditPlan {
    let start = selection.0.min(selection.1);
    let end = selection.0.max(selection.1);
    let line = line_start(source, start);
    let prefix = if indent {
        source[line..start]
            .bytes()
            .take_while(|b| matches!(b, b' ' | b'\t'))
            .count()
    } else {
        0
    };
    let crlf = source[line..]
        .find('\n')
        .map(|offset| line + offset)
        .or_else(|| source[..line].rfind('\n'))
        .is_some_and(|pos| pos > 0 && source.as_bytes()[pos - 1] == b'\r');
    let text = format!(
        "{}{}",
        if crlf { "\r\n" } else { "\n" },
        &source[line..line + prefix]
    );
    let cursor = start + text.len();
    EditPlan {
        edits: vec![Edit {
            range: start..end,
            text,
        }],
        selection: (cursor, cursor),
    }
}
pub(super) fn brackets(selection: (usize, usize), open: char) -> Option<EditPlan> {
    let close = match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        _ => return None,
    };
    let start = selection.0.min(selection.1);
    let end = selection.0.max(selection.1);
    let edits = if start == end {
        vec![Edit {
            range: start..start,
            text: format!("{open}{close}"),
        }]
    } else {
        vec![
            Edit {
                range: start..start,
                text: open.to_string(),
            },
            Edit {
                range: end..end,
                text: close.to_string(),
            },
        ]
    };
    Some(EditPlan {
        edits,
        selection: (selection.0 + 1, selection.1 + 1),
    })
}
pub(super) fn indent(
    source: &str,
    selection: (usize, usize),
    width: usize,
    backwards: bool,
) -> Option<EditPlan> {
    // A malformed setting must not allocate an unbounded keypress-sized indent.
    let width = width.clamp(1, 256);
    let start = selection.0.min(selection.1);
    let end = selection.0.max(selection.1);
    let first = line_start(source, start);
    let last = line_start(
        source,
        if end > start && source.as_bytes()[end - 1] == b'\n' {
            end - 1
        } else {
            end
        },
    );
    if !backwards && first == last {
        let cursor = start + width;
        return Some(EditPlan {
            edits: vec![Edit {
                range: start..end,
                text: " ".repeat(width),
            }],
            selection: (cursor, cursor),
        });
    }
    let mut starts = vec![first];
    starts.extend(
        source[first..last]
            .match_indices('\n')
            .map(|(offset, _)| first + offset + 1),
    );
    let mut edits = Vec::new();
    for position in starts {
        let rest = &source[position..];
        if backwards {
            let spaces = rest.bytes().take_while(|b| *b == b' ').count();
            let remove = if spaces > 0 {
                let remainder = spaces % width;
                if remainder == 0 {
                    spaces.min(width)
                } else {
                    remainder
                }
            } else {
                usize::from(rest.starts_with('\t'))
            };
            if remove > 0 {
                edits.push(Edit {
                    range: position..position + remove,
                    text: String::new(),
                });
            }
        } else if !rest.is_empty() && !rest.starts_with(['\r', '\n']) {
            edits.push(Edit {
                range: position..position,
                text: " ".repeat(width),
            });
        }
    }
    if edits.is_empty() {
        return None;
    }
    let selection = mapped_selection(&edits, selection);
    Some(EditPlan { edits, selection })
}

fn mapped_selection(edits: &[Edit], selection: (usize, usize)) -> (usize, usize) {
    let start = selection.0.min(selection.1);
    let map = |index: usize| {
        let mut removed = 0;
        let mut inserted = 0;
        for edit in edits {
            if index < edit.range.start {
                break;
            }
            if edit.range.is_empty() {
                if index > edit.range.start || index != start {
                    inserted += edit.text.len();
                }
            } else if index < edit.range.end {
                return edit.range.start - removed + inserted;
            } else {
                removed += edit.range.len();
                inserted += edit.text.len();
            }
        }
        index - removed + inserted
    };
    (map(selection.0), map(selection.1))
}
pub(super) fn comment(source: &str, selection: (usize, usize), prefix: &str) -> Option<EditPlan> {
    if prefix.is_empty() || prefix.contains(['\r', '\n']) {
        return None;
    }
    let start = selection.0.min(selection.1);
    let end = selection.0.max(selection.1);
    let first = line_start(source, start);
    let last = line_start(
        source,
        if end > start && source.as_bytes()[end - 1] == b'\n' {
            end - 1
        } else {
            end
        },
    );
    let starts = std::iter::once(first)
        .chain(
            source[first..last]
                .match_indices('\n')
                .map(|(index, _)| first + index + 1),
        )
        .collect::<Vec<_>>();
    let lines = starts
        .iter()
        .map(|start| {
            let end = source[*start..]
                .find('\n')
                .map_or(source.len(), |index| *start + index);
            (*start, &source[*start..end])
        })
        .collect::<Vec<_>>();
    let uncomment = lines.iter().any(|(_, line)| !line.trim().is_empty())
        && lines
            .iter()
            .filter(|(_, line)| !line.trim().is_empty())
            .all(|(_, line)| line.trim_start_matches([' ', '\t']).starts_with(prefix));
    let mut edits = Vec::new();
    for (start, line) in lines {
        if line.trim().is_empty() && starts.len() > 1 {
            continue;
        }
        let trimmed = line.trim_start_matches([' ', '\t']);
        if uncomment && trimmed.starts_with(prefix) {
            let position = start + line.len() - trimmed.len();
            edits.push(Edit {
                range: position..position + prefix.len(),
                text: String::new(),
            });
        } else if !uncomment {
            edits.push(Edit {
                range: start..start,
                text: prefix.to_owned(),
            });
        }
    }
    if edits.is_empty() {
        return None;
    }
    Some(EditPlan {
        selection: mapped_selection(&edits, selection),
        edits,
    })
}
