//! Presentation snapshots own current authored sources, not editing history.
use super::*;

impl IsoPaintLayer {
    /// The authoring owner makes derived paint current before taking a view
    /// snapshot. Obsolete experimental versions are not replayed by a renderer.
    pub fn presentation_source(&self, selection: bool) -> Self {
        let mut source = Self {
            visible: self.visible,
            chunk_size: self.chunk_size,
            baked_version: ISO_PAINT_BAKE_VERSION,
            selection_visible: selection,
            surface_instance_owners: self.surface_instance_owners.clone(),
            ..Default::default()
        };
        if (self.visible || selection && self.active_selection().is_some())
            && self.baked_version == ISO_PAINT_BAKE_VERSION
        {
            // Selection rendering retains the same alpha-routing source even
            // when the authored paint itself is hidden.
            source.baked_chunks = self.baked_chunks.clone();
        }
        if self.visible {
            for (key, chunk) in &self.chunks {
                if !chunk.stamps.is_empty() {
                    source.chunks.insert(
                        key.clone(),
                        IsoPaintChunk {
                            origin: chunk.origin,
                            revision: chunk.revision,
                            stamp_revision: chunk.stamp_revision,
                            strokes: Vec::new(),
                            stamps: chunk.stamps.clone(),
                        },
                    );
                }
            }
        }
        if selection && let Some(active) = self.active_selection() {
            source.active_selection_id = Some(active.id);
            source.selection_masks.insert(active.id, active.clone());
        }
        source
    }
}
