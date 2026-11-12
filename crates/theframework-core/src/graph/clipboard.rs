//! Portable graph clipboard data and immutable command context. Applications
//! define compatible graph kinds and own all mutations/platform requests.
use super::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphClipboardAction {
    Copy,
    Cut,
    Paste,
}
#[derive(Clone, Debug)]
pub struct GraphClipboardIntent {
    pub binding: u64,
    pub serial: u64,
    pub action: GraphClipboardAction,
    pub document: Arc<GraphDocument>,
    pub nodes: Vec<GraphId>,
    pub selected: Option<GraphId>,
    pub connection: Option<GraphId>,
    pub position: Point,
}
impl GraphClipboardIntent {
    /// Copy only the addressed nodes and the wires whose endpoints are copied.
    pub fn selection(&self) -> GraphDocument {
        let mut document = self.document.as_ref().clone();
        document.nodes.retain(|node| self.nodes.contains(&node.id));
        let ports: std::collections::HashSet<_> = document
            .nodes
            .iter()
            .flat_map(|node| node.ports.iter().map(|port| port.id))
            .collect();
        document
            .connections
            .retain(|wire| ports.contains(&wire.from) && ports.contains(&wire.to));
        document
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphClipboard {
    format: String,
    version: u32,
    pub kind: String,
    pub document: GraphDocument,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphClipboardError {
    TooLarge,
    Invalid,
    UnsupportedVersion,
    WrongKind,
}
impl std::fmt::Display for GraphClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TooLarge => "The clipboard graph exceeds the size limit",
            Self::Invalid => "The clipboard does not contain a supported graph",
            Self::UnsupportedVersion => "The clipboard graph version is not supported",
            Self::WrongKind => "The clipboard belongs to a different kind of graph",
        })
    }
}
impl std::error::Error for GraphClipboardError {}
impl GraphClipboard {
    pub fn new(kind: impl Into<String>, document: GraphDocument) -> Self {
        Self {
            format: "theframework.graph".into(),
            version: 1,
            kind: kind.into(),
            document,
        }
    }
    pub fn encode(&self, byte_limit: usize) -> Result<String, GraphClipboardError> {
        self.validate(&self.kind)?;
        let text = serde_json::to_string(self).map_err(|_| GraphClipboardError::Invalid)?;
        if text.len() > byte_limit {
            return Err(GraphClipboardError::TooLarge);
        }
        Ok(text)
    }
    pub fn decode(text: &str, kind: &str, byte_limit: usize) -> Result<Self, GraphClipboardError> {
        if text.len() > byte_limit {
            return Err(GraphClipboardError::TooLarge);
        }
        let value: Self = serde_json::from_str(text).map_err(|_| GraphClipboardError::Invalid)?;
        value.validate(kind)?;
        Ok(value)
    }
    fn validate(&self, kind: &str) -> Result<(), GraphClipboardError> {
        if self.format != "theframework.graph" {
            return Err(GraphClipboardError::Invalid);
        }
        if self.version != 1 || self.document.version != 1 {
            return Err(GraphClipboardError::UnsupportedVersion);
        }
        if self.kind != kind {
            return Err(GraphClipboardError::WrongKind);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graph_clipboard_roundtrips_exact_source_and_rejects_other_formats_kinds_versions_and_oversize()
     {
        let mut node = GraphNode::new("Native", [12.25, 33.5], [1, 2, 3, 255]);
        node.rows.push(GraphRow::new(
            "Source",
            GraphControlValue::Custom {
                kind: "source".into(),
                data: serde_json::json!({"text":"東京\r\nمرحبا e\u{301}","future":{"value":42}}),
            },
        ));
        let clipboard = GraphClipboard::new(
            "behavior",
            GraphDocument {
                nodes: vec![node],
                ..Default::default()
            },
        );
        let text = clipboard.encode(4096).unwrap();
        assert_eq!(
            GraphClipboard::decode(&text, "behavior", 4096).unwrap(),
            clipboard
        );
        assert_eq!(
            GraphClipboard::decode(&text, "configuration", 4096).unwrap_err(),
            GraphClipboardError::WrongKind
        );
        assert_eq!(
            GraphClipboard::decode(&text, "behavior", 4).unwrap_err(),
            GraphClipboardError::TooLarge
        );
        let mut value = serde_json::to_value(&clipboard).unwrap();
        value["version"] = 2.into();
        assert_eq!(
            GraphClipboard::decode(&value.to_string(), "behavior", 4096).unwrap_err(),
            GraphClipboardError::UnsupportedVersion
        );
        value["version"] = 1.into();
        value["document"]["version"] = 2.into();
        assert_eq!(
            GraphClipboard::decode(&value.to_string(), "behavior", 4096).unwrap_err(),
            GraphClipboardError::UnsupportedVersion
        );
        value["format"] = "other".into();
        assert_eq!(
            GraphClipboard::decode(&value.to_string(), "behavior", 4096).unwrap_err(),
            GraphClipboardError::Invalid
        );
    }
}
