//! Presentation snapshots own current authored sources, not editing history.
use super::*;

impl IsoPaintLayer {
    /// Compare only the original backing and metadata copied by
    /// presentation_source. This never walks pixels or clones authoring history.
    pub fn same_presentation_source(&self, source: &Self, selection: bool) -> bool {
        if self.visible != source.visible
            || self.chunk_size != source.chunk_size
            || source.selection_visible != selection
            || !self
                .surface_instance_owners
                .same_storage(&source.surface_instance_owners)
        {
            return false;
        }
        let active = selection.then(|| self.active_selection()).flatten();
        if active != source.active_selection() {
            return false;
        }
        let current =
            (self.visible || active.is_some()) && self.baked_version == ISO_PAINT_BAKE_VERSION;
        if current {
            if !(self.baked_chunks.is_empty() && source.baked_chunks.is_empty())
                && !self.baked_chunks.same_storage(&source.baked_chunks)
            {
                return false;
            }
        } else if !source.baked_chunks.is_empty() {
            return false;
        }
        let mut chunks = self
            .chunks
            .iter()
            .filter(|(_, chunk)| self.visible && !chunk.stamps.is_empty());
        source.chunks.iter().all(|(key, source)| {
            chunks.next().is_some_and(|(current_key, current)| {
                key == current_key
                    && source.origin == current.origin
                    && source.revision == current.revision
                    && source.stamp_revision == current.stamp_revision
                    && source.stamps.same_storage(&current.stamps)
            })
        }) && chunks.next().is_none()
    }
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
