//! Markdown projection is document data: text plus grapheme spans, without
//! renderer/font/window access. Link semantics survive nested emphasis.
use super::TheMarkdownNode;
use markdown::{ParseOptions, mdast::Node};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
#[derive(Default)]
struct Projection {
    text: String,
    spans: Vec<(Range<usize>, TheMarkdownNode)>,
}
impl Projection {
    fn append(&mut self, text: &str, kind: &TheMarkdownNode) {
        if text.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(text);
        self.spans.push((start..self.text.len(), kind.clone()));
    }
    fn newline(&mut self) {
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.append("\n", &TheMarkdownNode::Text);
        }
    }
    fn visit(&mut self, node: &Node, inherited: &TheMarkdownNode) {
        let kind = match node {
            Node::Link(link) => TheMarkdownNode::Link(link.url.clone()),
            _ if inherited.is_link() => inherited.clone(),
            Node::Heading(_) => TheMarkdownNode::Heading,
            Node::Strong(_) if !matches!(inherited, TheMarkdownNode::Heading) => {
                TheMarkdownNode::Strong
            }
            Node::Emphasis(_) if matches!(inherited, TheMarkdownNode::Text) => {
                TheMarkdownNode::Emphasis
            }
            _ => inherited.clone(),
        };
        match node {
            Node::Text(text) => self.append(&text.value, &kind),
            Node::InlineCode(code) => self.append(&code.value, &kind),
            Node::Code(code) => {
                self.newline();
                self.append(&code.value, &kind);
                self.newline();
            }
            Node::Break(_) => self.append("\n", &TheMarkdownNode::Text),
            _ => {
                if let Some(children) = node.children() {
                    for child in children {
                        self.visit(child, &kind);
                    }
                }
            }
        }
        if matches!(node, Node::Heading(_) | Node::Paragraph(_)) {
            self.newline();
        }
    }
}
pub(super) fn project(
    source: &str,
) -> Result<(String, Vec<(Range<usize>, TheMarkdownNode)>), String> {
    let tree = markdown::to_mdast(source, &ParseOptions::default()).map_err(|e| e.to_string())?;
    let mut projection = Projection::default();
    projection.visit(&tree, &TheMarkdownNode::Text);
    while projection.text.ends_with('\n') {
        projection.text.pop();
    }
    let mut bytes: Vec<_> = projection
        .text
        .grapheme_indices(true)
        .map(|v| v.0)
        .collect();
    bytes.push(projection.text.len());
    let mut spans: Vec<(Range<usize>, TheMarkdownNode)> = Vec::new();
    for (range, kind) in projection.spans {
        let start_byte = range.start.min(projection.text.len());
        let end_byte = range.end.min(projection.text.len());
        let start = bytes
            .partition_point(|b| *b <= start_byte)
            .saturating_sub(1);
        let end = bytes.partition_point(|b| *b < end_byte);
        if start >= end {
            continue;
        }
        if let Some((last, last_kind)) = spans.last_mut() {
            if *last_kind == kind && last.end == start {
                last.end = end;
                continue;
            }
        }
        spans.push((start..end, kind));
    }
    Ok((projection.text, spans))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_after_unicode_and_inside_emphasis_use_grapheme_ranges() {
        let (text, spans) = project("é **[cafe\u{301}](https://example.invalid)** tail").unwrap();
        assert_eq!(text, "é cafe\u{301} tail");
        let (range, _) = spans.iter().find(|(_, kind)| kind.is_link()).unwrap();
        assert_eq!(range.clone(), 2..6);
    }
    #[test]
    fn code_and_paragraph_text_are_preserved_without_markup() {
        let (text, _) = project("# Heading\n\nUse `value`.\n\n```\nline();\n```").unwrap();
        assert_eq!(text, "Heading\nUse value.\nline();");
    }
}
