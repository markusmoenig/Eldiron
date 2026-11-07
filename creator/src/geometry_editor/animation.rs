use super::*;
use denrim_materials::geometry::{Mesh, rig::Motion};
use std::time::Instant;
pub struct Playback {
    pub clips: Vec<(String, String)>,
    pub scope: Vec<String>,
    motions: std::collections::BTreeMap<String, Motion>,
    framing_dirty: bool,
    pub selected: usize,
    pub phase: f32,
    pub playing: bool,
    pub show_rig: bool,
    pub framing: Option<([f32; 3], [f32; 3])>,
    last: Option<Instant>,
    event_cursor: Option<(usize, f32)>,
}
impl Default for Playback {
    fn default() -> Self {
        Self {
            clips: vec![],
            scope: vec![],
            motions: Default::default(),
            framing_dirty: false,
            selected: 0,
            phase: 0.,
            playing: false,
            show_rig: true,
            framing: None,
            last: None,
            event_cursor: None,
        }
    }
}
impl Playback {
    pub fn suspend(&mut self) {
        self.last = None;
    }
    pub fn inspect(&mut self, doc: &Document, operation: &str, selection: &rigging::Selection) {
        for row in &doc.rows {
            if let Some(tile) = row.tiles.iter().find(|t| t.id == operation) {
                if let Operation::Motion { tracks, .. } = &tile.op {
                    self.selected = self
                        .clips
                        .iter()
                        .position(|(id, _)| id == &row.id)
                        .map_or(0, |i| i + 1);
                    self.phase = tracks
                        .get(selection.track.min(tracks.len().saturating_sub(1)))
                        .and_then(|t| t.keys.get(selection.key.min(t.keys.len() - 1)))
                        .map_or(0., |k| k.time);
                    self.playing = false;
                    self.last = None;
                }
                break;
            }
        }
    }
    pub fn refresh(
        &mut self,
        root: &Document,
        path: &[String],
        e: &Evaluation,
        reset: bool,
        ui: &mut TheUI,
    ) -> Result<(), String> {
        let previous = self
            .selected
            .checked_sub(1)
            .and_then(|i| self.clips.get(i))
            .map(|(id, _)| id.clone());
        let depth = (0..=path.len())
            .rev()
            .find(|&n| {
                root.group_document(&path[..n]).is_ok_and(|d| {
                    d.rows
                        .iter()
                        .flat_map(|r| &r.tiles)
                        .any(|t| matches!(t.op, Operation::Motion { .. }))
                })
            })
            .unwrap_or(path.len());
        self.scope = path[..depth].to_vec();
        let doc = root
            .group_document(&self.scope)
            .map_err(|e| e.to_string())?;
        let external = if depth != path.len() {
            Some(doc.evaluate().map_err(|e| e.to_string())?)
        } else {
            None
        };
        let source = external.as_ref().unwrap_or(e);
        self.motions.clear();
        let mut clips = vec![];
        if let Some(rig) = &e.mesh.rig {
            for row in &doc.rows {
                if let Some(Value::Motion(m)) = source.rows.get(&row.id) {
                    // Preview ancestor clips on the editable base rig using stable joint IDs.
                    // Authored motion data is untouched; unrelated skeletons are filtered out.
                    if m.rig.joints.iter().all(|j| {
                        rig.index(&j.id).is_ok_and(|i| {
                            let parent =
                                |r: &denrim_materials::geometry::rig::Rig, p: Option<usize>| {
                                    p.map(|i| r.joints[i].id.clone())
                                };
                            parent(rig, rig.joints[i].parent) == parent(&m.rig, j.parent)
                        })
                    }) {
                        let mut motion = m.clone();
                        motion.rig = rig.clone();
                        if motion.validate().is_ok() {
                            clips.push((row.id.clone(), row.label.clone()));
                            self.motions.insert(row.id.clone(), motion);
                        }
                    }
                }
            }
        }
        self.selected = if reset {
            self.phase = 0.;
            self.playing = false;
            self.last = None;
            0
        } else {
            previous
                .and_then(|id| clips.iter().position(|(k, _)| k == &id))
                .map_or(0, |i| i + 1)
        };
        if self.selected == 0 {
            self.playing = false;
        }
        if clips != self.clips || reset {
            if let Some(menu) = ui
                .get_widget("MZ Motion")
                .and_then(|w| w.as_drop_down_menu())
            {
                menu.clear_options();
                menu.add_option("Rest pose".into());
                for (_, label) in &clips {
                    menu.add_option(label.clone());
                }
                menu.set_selected_index(self.selected as i32);
            }
        }
        self.clips = clips;
        self.framing_dirty = true;
        self.update_framing(e);
        Ok(())
    }
    pub fn motion<'a>(&'a self, _e: &Evaluation) -> Option<&'a Motion> {
        self.selected
            .checked_sub(1)
            .and_then(|i| self.clips.get(i))
            .and_then(|(id, _)| self.motions.get(id))
    }
    fn update_framing(&mut self, e: &Evaluation) {
        if self.framing_dirty {
            self.framing = Some(bounds(
                &e.mesh,
                &self.motion(e).into_iter().collect::<Vec<_>>(),
            ));
            self.framing_dirty = false;
        }
    }
    pub fn select_row(&mut self, row: &str) {
        self.selected = self
            .clips
            .iter()
            .position(|(id, _)| id == row)
            .map_or(0, |i| i + 1);
        self.framing_dirty = true;
    }
    pub fn rest(&mut self) {
        self.selected = 0;
        self.phase = 0.;
        self.playing = false;
        self.last = None;
        self.framing_dirty = true;
    }
    pub fn tick(&mut self, e: &Evaluation, ui: &mut TheUI, ctx: &mut TheContext) {
        self.update_framing(e);
        let now = Instant::now();
        let elapsed = self
            .last
            .replace(now)
            .map_or(0., |last| (now - last).as_secs_f32());
        let was_playing = self.playing;
        let before = self.phase;
        if self.playing {
            if let Some(m) = self.motion(e) {
                let duration = m.duration;
                let looping = m.looping;
                self.phase += elapsed / duration;
                if looping {
                    self.phase = self.phase.rem_euclid(1.);
                } else if self.phase >= 1. {
                    self.phase = 1.;
                    self.playing = false;
                }
            } else {
                self.playing = false;
            }
        }
        if was_playing {
            if let Some(m) = self.motion(e) {
                let start = self
                    .event_cursor
                    .filter(|(clip, _)| *clip == self.selected)
                    .map_or(before, |(_, phase)| phase);
                let end = if m.looping {
                    start + elapsed / m.duration
                } else {
                    self.phase
                };
                for event in m.events_between(start * m.duration, end * m.duration) {
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        format!("Animation event: {}", event.name),
                    ));
                }
            }
        }
        self.event_cursor = Some((self.selected, self.phase));
        if let Some(w) = ui
            .get_widget("MZ Motion Play")
            .and_then(|w| w.as_group_button())
        {
            if w.index() != i32::from(self.playing) {
                w.set_index(i32::from(self.playing));
            }
        }
        layout::set_choice(ui, "MZ Motion", self.selected);
        for name in ["MZ Motion Play", "MZ Motion Phase"] {
            if let Some(w) = ui.get_widget(name) {
                let disabled = self.clips.is_empty();
                if w.disabled() != disabled {
                    w.set_disabled(disabled);
                }
            }
        }
        // Do not replace a value the user is entering manually.
        if ctx
            .ui
            .focus
            .as_ref()
            .is_none_or(|id| id.name != "MZ Motion Phase")
        {
            if let Some(w) = ui.get_widget("MZ Motion Phase") {
                if w.value().to_f32() != Some(self.phase) {
                    w.set_value(TheValue::Float(self.phase));
                }
            }
        }
    }
    pub fn event(&mut self, event: &TheEvent) -> bool {
        match event {
            TheEvent::IndexChanged(id, i) if id.name == "MZ Motion" => {
                self.selected = (*i as usize).min(self.clips.len());
                self.framing_dirty = true;
                self.phase = 0.;
                self.last = None;
                self.playing = self.selected > 0;
                true
            }
            TheEvent::IndexChanged(id, i) if id.name == "MZ Motion Play" => {
                // Play from Rest starts the first available clip instead of
                // immediately snapping the transport back to Pause on the next tick.
                if *i == 1 && self.selected == 0 && !self.clips.is_empty() {
                    self.selected = 1;
                    self.phase = 0.;
                    self.framing_dirty = true;
                }
                self.playing = *i == 1 && self.selected > 0;
                if self.phase >= 1. {
                    self.phase = 0.;
                }
                self.last = None;
                true
            }
            TheEvent::ValueChanged(id, value) if id.name == "MZ Motion Phase" => {
                if let Some(v) = value.to_f32() {
                    self.phase = v.clamp(0., 1.);
                    self.playing = false;
                }
                true
            }
            TheEvent::IndexChanged(id, i) if id.name == "MZ Rig Overlay" => {
                self.show_rig = *i == 1;
                true
            }
            _ => false,
        }
    }
}
pub fn bounds(mesh: &Mesh, motions: &[&Motion]) -> ([f32; 3], [f32; 3]) {
    let (mut lo, mut hi) = mesh.bounds();
    for motion in motions {
        for i in 0..=16 {
            if let Ok(posed) = mesh.posed(Some(motion), motion.duration * i as f32 / 16.) {
                let (a, b) = posed.mesh.bounds();
                for k in 0..3 {
                    lo[k] = lo[k].min(a[k]);
                    hi[k] = hi[k].max(b[k]);
                }
            }
        }
    }
    (lo, hi)
}
