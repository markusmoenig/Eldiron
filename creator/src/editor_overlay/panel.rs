//! Presented-frame ownership shared by interactive editor panels. Preparation
//! freezes actions but does not activate them. Native roots acknowledge only
//! receipts included in their successful surface presentation.
use super::*;
use crate::hud::HudColors;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PanelScope {
    pub(crate) document: Uuid,
    pub(crate) tool: Uuid,
    pub(crate) revision: u64,
}
#[derive(PartialEq)]
struct Source<S> {
    scope: PanelScope,
    size: (usize, usize),
    state: S,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PanelEvent {
    Unavailable,
    Outside,
    Consumed,
}
pub(crate) enum PanelHit<A> {
    Unavailable,
    Outside,
    Inside(Option<(TheDim, A)>),
}
pub(crate) struct PanelFrame<S, A> {
    pub(crate) commands: EditorOverlay,
    source: Arc<Source<S>>,
    pub(crate) bounds: TheDim,
    pub(crate) buttons: Vec<(TheDim, A)>,
    owner: Uuid,
    serial: u64,
    #[cfg(feature = "native-ui")]
    receipt_owner: u64,
}
impl<S, A> PanelFrame<S, A> {
    #[cfg(feature = "native-ui")]
    pub(crate) fn receipt(&self) -> theframework::paint::FrameReceipt {
        theframework::paint::FrameReceipt {
            owner: self.receipt_owner,
            serial: self.serial,
        }
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn paint(
        &self,
        text: &mut theframework::text::TextSystem,
        images: &mut dyn theframework::theui::paint::UiImageResources,
        density: f32,
    ) -> Result<theframework::paint::PaintList, theframework::theui::paint::UiPaintError> {
        let mut list = self.commands.paint(text, images, density)?;
        list.ops
            .push(theframework::paint::PaintOp::Receipt(self.receipt()));
        Ok(list)
    }
}
pub(crate) struct PanelFrames<S, A> {
    owner: Uuid,
    serial: u64,
    invalidated_after: u64,
    last_presented: u64,
    prepared: Option<Arc<Source<S>>>,
    displayed: Option<(Arc<Source<S>>, TheDim, Vec<(TheDim, A)>)>,
    #[cfg(feature = "native-ui")]
    receipt_owner: u64,
}
impl<S, A> Default for PanelFrames<S, A> {
    fn default() -> Self {
        Self {
            owner: Uuid::new_v4(),
            serial: 0,
            invalidated_after: 0,
            last_presented: 0,
            prepared: None,
            displayed: None,
            #[cfg(feature = "native-ui")]
            receipt_owner: theframework::paint::new_receipt_owner(),
        }
    }
}
impl<S: PartialEq, A: Clone> PanelFrames<S, A> {
    pub(crate) fn owner(&self) -> Uuid {
        self.owner
    }
    pub(crate) fn invalidate(&mut self) {
        self.invalidated_after = self.serial;
        self.prepared = None;
        self.displayed = None;
    }
    fn stage(&mut self, scope: PanelScope, state: S, builder: PanelBuilder<A>) -> PanelFrame<S, A> {
        let source = Arc::new(Source {
            scope,
            size: builder.commands.size,
            state,
        });
        if self
            .prepared
            .as_ref()
            .is_some_and(|old| old.as_ref() != source.as_ref())
        {
            self.invalidate();
        }
        self.serial = self
            .serial
            .checked_add(1)
            .expect("editor panel serial exhausted");
        self.prepared = Some(source.clone());
        PanelFrame {
            commands: builder.commands,
            source,
            bounds: builder.bounds,
            buttons: builder.buttons,
            owner: self.owner,
            serial: self.serial,
            #[cfg(feature = "native-ui")]
            receipt_owner: self.receipt_owner,
        }
    }
    pub(crate) fn acknowledge_reference(
        &mut self,
        frame: &PanelFrame<S, A>,
        scope: PanelScope,
        size: (usize, usize),
        state: &S,
    ) -> bool {
        if frame.commands.is_empty()
            || frame.owner != self.owner
            || frame.serial <= self.invalidated_after
            || frame.serial <= self.last_presented
            || frame.serial > self.serial
            || frame.source.scope != scope
            || frame.source.size != size
            || &frame.source.state != state
            || self
                .prepared
                .as_ref()
                .is_none_or(|current| current.as_ref() != frame.source.as_ref())
        {
            return false;
        }
        self.last_presented = frame.serial;
        self.displayed = Some((frame.source.clone(), frame.bounds, frame.buttons.clone()));
        true
    }
    #[cfg(feature = "native-ui")]
    pub(crate) fn presented_paint(
        &mut self,
        frame: &PanelFrame<S, A>,
        receipts: &[theframework::paint::FrameReceipt],
        scope: PanelScope,
        size: (usize, usize),
        state: &S,
    ) -> bool {
        frame.receipt_owner == self.receipt_owner
            && receipts.contains(&frame.receipt())
            && self.acknowledge_reference(frame, scope, size, state)
    }
    pub(crate) fn hit_test(
        &self,
        scope: PanelScope,
        size: (usize, usize),
        state: &S,
        point: Vec2<i32>,
    ) -> PanelHit<A> {
        let Some((source, bounds, buttons)) = &self.displayed else {
            return PanelHit::Unavailable;
        };
        if source.scope != scope || source.size != size || &source.state != state {
            return PanelHit::Unavailable;
        }
        if point.x < 0
            || point.y < 0
            || point.x as usize >= size.0
            || point.y as usize >= size.1
            || !bounds.contains(point)
        {
            return PanelHit::Outside;
        }
        PanelHit::Inside(
            buttons
                .iter()
                .find(|(rect, _)| rect.contains(point))
                .cloned(),
        )
    }
}

pub(crate) struct PanelBuilder<A> {
    pub(crate) commands: EditorOverlay,
    pub(crate) bounds: TheDim,
    pub(crate) buttons: Vec<(TheDim, A)>,
    pub(crate) colors: HudColors,
}
impl<A: Clone> PanelBuilder<A> {
    pub(crate) fn new(size: (usize, usize), colors: HudColors) -> Self {
        Self {
            commands: EditorOverlay::new(size),
            bounds: TheDim::default(),
            buttons: vec![],
            colors,
        }
    }
    pub(crate) fn button(&mut self, rect: TheDim, label: &str, selected: bool, action: A) {
        let c = self.colors;
        let rect_usize = box_rect(rect);
        self.commands
            .rect(&rect_usize, &if selected { c.selected } else { c.panel });
        self.commands
            .rect_outline(&rect_usize, &if selected { c.accent } else { c.border });
        self.commands.text_rect_blend(
            &(
                rect.x as usize + 4,
                rect.y as usize,
                rect.width as usize - 8,
                rect.height as usize,
            ),
            label,
            TheFontSettings {
                size: 13.,
                ..Default::default()
            },
            &c.selected_text,
            TheHorizontalAlign::Center,
            TheVerticalAlign::Center,
        );
        self.buttons.push((rect, action));
    }
    pub(crate) fn finish<S: PartialEq>(
        self,
        frames: &mut PanelFrames<S, A>,
        scope: PanelScope,
        state: S,
    ) -> PanelFrame<S, A> {
        frames.stage(scope, state, self)
    }
}
pub(crate) fn box_rect(rect: TheDim) -> BoxRect {
    (
        rect.x.max(0) as usize,
        rect.y.max(0) as usize,
        rect.width.max(0) as usize,
        rect.height.max(0) as usize,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scope() -> PanelScope {
        PanelScope {
            document: Uuid::new_v4(),
            tool: Uuid::new_v4(),
            revision: 5,
        }
    }
    fn prepare(
        owner: &mut PanelFrames<u32, usize>,
        scope: PanelScope,
        source: u32,
    ) -> PanelFrame<u32, usize> {
        let mut builder = PanelBuilder::new((20, 20), crate::hud::colors());
        builder.bounds = TheDim::rect(0, 0, 30, 30);
        builder.commands.rect(&(0, 0, 30, 30), &[40, 50, 60, 255]);
        builder.buttons.push((TheDim::rect(0, 0, 30, 30), 3));
        builder.finish(owner, scope, source)
    }
    #[test]
    fn panel_frames_reject_reordered_sources_foreign_owners_and_outside_viewport_hits() {
        let mut owner = PanelFrames::default();
        let scope = scope();
        let point = Vec2::new(2, 2);
        let first = prepare(&mut owner, scope, 1);
        let equivalent = prepare(&mut owner, scope, 1);
        assert!(owner.acknowledge_reference(&first, scope, (20, 20), &1));
        assert!(owner.acknowledge_reference(&equivalent, scope, (20, 20), &1));
        assert!(!owner.acknowledge_reference(&first, scope, (20, 20), &1));
        assert!(matches!(
            owner.hit_test(scope, (20, 20), &1, point),
            PanelHit::Inside(Some((_, 3)))
        ));
        for point in [Vec2::new(-1, 1), Vec2::new(20, 1), Vec2::new(1, 20)] {
            assert!(matches!(
                owner.hit_test(scope, (20, 20), &1, point),
                PanelHit::Outside
            ));
        }
        let changed = prepare(&mut owner, scope, 2);
        assert!(matches!(
            owner.hit_test(scope, (20, 20), &1, point),
            PanelHit::Unavailable
        ));
        let restored = prepare(&mut owner, scope, 1);
        assert!(!owner.acknowledge_reference(&equivalent, scope, (20, 20), &1));
        assert!(!owner.acknowledge_reference(&changed, scope, (20, 20), &1));
        assert!(owner.acknowledge_reference(&restored, scope, (20, 20), &1));
        let mut foreign = PanelFrames::default();
        let foreign_frame = prepare(&mut foreign, scope, 1);
        assert!(!owner.acknowledge_reference(&foreign_frame, scope, (20, 20), &1));
        assert!(!foreign.acknowledge_reference(&restored, scope, (20, 20), &1));
    }
    #[test]
    #[cfg(feature = "native-ui")]
    fn native_panel_acknowledgement_requires_its_own_presented_root_receipt() {
        let mut owner = PanelFrames::default();
        let scope = scope();
        let first = prepare(&mut owner, scope, 1);
        let mut foreign = PanelFrames::default();
        let other = prepare(&mut foreign, scope, 1);
        assert!(!owner.presented_paint(&first, &[], scope, (20, 20), &1));
        assert!(!owner.presented_paint(&first, &[other.receipt()], scope, (20, 20), &1));
        assert!(!owner.presented_paint(&other, &[other.receipt()], scope, (20, 20), &1));
        let mut list = theframework::paint::PaintList::default();
        list.ops
            .push(theframework::paint::PaintOp::Receipt(first.receipt()));
        assert!(owner.presented_paint(
            &first,
            &list.receipts().collect::<Vec<_>>(),
            scope,
            (20, 20),
            &1
        ));
        owner.invalidate();
        assert!(!owner.presented_paint(&first, &[first.receipt()], scope, (20, 20), &1));
        let next = prepare(&mut owner, scope, 1);
        assert!(owner.presented_paint(&next, &[next.receipt()], scope, (20, 20), &1));
    }
}
