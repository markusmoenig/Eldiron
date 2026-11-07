use super::*;
use denrim_materials::geometry::{
    authoring,
    group::Modifier,
    rig::{Key, Track},
};
use scene::{Drag, Handle};
impl GeometryEditor {
    fn scene_commit_root(
        &mut self,
        root: Document,
        project: &mut Project,
        ctx: &mut TheContext,
    ) -> Result<(), String> {
        let id = self.selected.ok_or("Select a model")?;
        let next = root.save().map_err(|e| e.to_string())?;
        let old = project.procedural_appearances.insert(id, next.clone());
        if old.as_ref() != Some(&next) {
            UNDOMANAGER.write().unwrap().add_undo(
                ProjectUndoAtom::ProceduralAppearanceEdit(id, old, Some(next)),
                ctx,
            );
        }
        self.local_edit = true;
        Ok(())
    }
    fn scene_root(&self, project: &Project) -> Result<Document, String> {
        Document::load(
            self.selected
                .and_then(|id| project.procedural_appearances.get(&id))
                .ok_or("Select a model")?,
        )
        .map_err(|e| e.to_string())
    }
    pub(super) fn scene_select(&mut self, operation: String) {
        self.selected_operation = Some(operation.clone());
        self.scene.select_after_refresh = Some(operation);
        self.inspector_dirty = true;
    }
    pub(super) fn finish_scene_drag(
        &mut self,
        project: &mut Project,
        ctx: &mut TheContext,
        cancel: bool,
    ) {
        if let Some(drag) = self.scene.drag.take() {
            if let Some(id) = self.selected {
                let after = project.procedural_appearances.get(&id).cloned();
                if cancel {
                    project.procedural_appearances.insert(id, drag.source);
                } else if after.as_ref() != Some(&drag.source) {
                    UNDOMANAGER.write().unwrap().add_undo(
                        ProjectUndoAtom::ProceduralAppearanceEdit(id, Some(drag.source), after),
                        ctx,
                    );
                }
                self.preview_key.clear();
            }
        }
    }
    fn scene_joint(&self, doc: &Document, e: &Evaluation) -> Option<String> {
        scene::armature(doc)
            .and_then(|(_, j)| j.get(self.inspector.joint))
            .map(|j| j.id.clone())
            .or_else(|| {
                e.mesh
                    .rig
                    .as_ref()
                    .and_then(|r| {
                        r.joints
                            .get(self.inspector.joint.min(r.joints.len().saturating_sub(1)))
                    })
                    .map(|j| j.id.clone())
            })
    }
    pub(super) fn scene_event(
        &mut self,
        event: &TheEvent,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Result<bool, String> {
        if let TheEvent::ModifierChanged(shift, _, alt, _) = event {
            self.scene.shift = *shift;
            self.scene.alt = *alt;
            return Ok(false);
        }
        if !self.active {
            return Ok(false);
        }
        match event {
            TheEvent::RenderViewScrollBy(id, d) | TheEvent::RenderViewPreciseScrollBy(id, d)
                if id.name == PREVIEW && self.scene.tool > 0 =>
            {
                if let Some(frame) = &self.scene.projection {
                    let (a, b) = frame.bounds;
                    let factor = (d.y as f32 * 0.01).exp().clamp(0.5, 2.);
                    let center: [f32; 3] = std::array::from_fn(|k| (a[k] + b[k]) / 2.);
                    let half: [f32; 3] =
                        std::array::from_fn(|k| ((b[k] - a[k]) / 2. * factor).clamp(0.025, 20.));
                    self.scene.view_bounds = Some((
                        std::array::from_fn(|k| center[k] - half[k]),
                        std::array::from_fn(|k| center[k] + half[k]),
                    ));
                    self.preview_key.clear();
                }
                return Ok(true);
            }
            TheEvent::IndexChanged(id, index) if id.name == "MZ Geometry Tool" => {
                self.finish_drag(project, ctx, false);
                self.scene.tool = *index as usize;
                if matches!(self.scene.tool, 2 | 3 | 5) {
                    self.playback.rest();
                    self.scene.posed.clear();
                }
                // Changing tools should not reset the user's camera or framing.
                // Contour editing alone requires a different projection.
                if self.camera == 3 && self.scene.tool > 0 {
                    self.camera = 0;
                }
                if self.scene.tool == 1 || self.scene.tool == 2 || self.scene.tool == 3 {
                    if let Some(root) = &self.root_document {
                        let path = root
                            .group_document(&self.group_path)
                            .ok()
                            .filter(|d| scene::armature(d).is_some())
                            .map(|_| self.group_path.clone())
                            .or_else(|| {
                                scene::find_path(root, |op| {
                                    matches!(op, Operation::Armature { .. })
                                })
                            });
                        if let Some(path) = path {
                            if self.group_path != path {
                                self.group_path = path;
                                self.key = None;
                                self.scene.view_bounds = None;
                            }
                            let doc = root
                                .group_document(&self.group_path)
                                .map_err(|e| e.to_string())?;
                            let op = if self.scene.tool == 3 {
                                scene::profiles(doc).map(|(id, _, _)| id)
                            } else {
                                scene::armature(doc).map(|(id, _)| id)
                            };
                            if let Some(op) = op {
                                self.scene.select_after_refresh = Some(op.into());
                                self.selected_operation = Some(op.into());
                                self.inspector_dirty = true;
                            }
                        }
                    }
                }
                if self.scene.tool == 4 {
                    let root = self.scene_root(project)?;
                    let path = root
                        .group_document(&self.group_path)
                        .ok()
                        .filter(|d| {
                            d.rows
                                .iter()
                                .flat_map(|r| &r.tiles)
                                .any(|t| matches!(t.op, Operation::Motion { .. }))
                        })
                        .map(|_| self.group_path.clone())
                        .or_else(|| {
                            scene::find_path(&root, |op| matches!(op, Operation::Motion { .. }))
                        });
                    if let Some(path) = path {
                        if path != self.group_path {
                            self.group_path = path;
                            self.key = None;
                            self.scene.view_bounds = None;
                        }
                        let doc = root
                            .group_document(&self.group_path)
                            .map_err(|e| e.to_string())?;
                        let wanted = self
                            .playback
                            .selected
                            .checked_sub(1)
                            .and_then(|i| self.playback.clips.get(i))
                            .map(|(id, _)| id);
                        let row = doc
                            .rows
                            .iter()
                            .filter(|r| {
                                r.tiles
                                    .iter()
                                    .any(|t| matches!(t.op, Operation::Motion { .. }))
                            })
                            .find(|r| Some(&r.id) == wanted)
                            .or_else(|| {
                                doc.rows.iter().find(|r| {
                                    r.tiles
                                        .iter()
                                        .any(|t| matches!(t.op, Operation::Motion { .. }))
                                })
                            })
                            .unwrap();
                        self.scene.pending_clip = Some(row.id.clone());
                        let tile = row
                            .tiles
                            .iter()
                            .find(|t| matches!(t.op, Operation::Motion { .. }))
                            .unwrap();
                        self.scene_select(tile.id.clone());
                    } else {
                        Self::status(ctx, "Use New Clip to create an animation");
                    }
                }
                if self.scene.tool == 5 {
                    let root = self.scene_root(project)?;
                    let doc = root
                        .group_document(&self.group_path)
                        .map_err(|e| e.to_string())?;
                    if !doc.rows.iter().flat_map(|r| &r.tiles).any(|t| {
                        matches!(
                            t.op,
                            Operation::Attach { .. }
                                | Operation::Grip { .. }
                                | Operation::ParticleEffect { .. }
                        )
                    }) {
                        if let Some(path) =
                            scene::find_path(&root, |op| matches!(op, Operation::Attach { .. }))
                        {
                            self.group_path = path;
                            self.key = None;
                            self.scene.view_bounds = None;
                        }
                    }
                }
                self.preview_key.clear();
                return Ok(true);
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == "MZ Add Motion" => {
                self.finish_drag(project, ctx, false);
                self.scene_add_motion(project, ctx)?;
                return Ok(true);
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "MZ Add Part"
                    || id.name == "MZ Mirror Branch"
                    || id.name == "MZ Delete Part"
                    || id.name == "MZ Add Profile"
                    || id.name == "MZ Delete Profile" =>
            {
                self.finish_drag(project, ctx, false);
                let mut root = self.scene_root(project)?;
                let doc = root
                    .group_document_mut(&self.group_path)
                    .map_err(|e| e.to_string())?;
                let e = doc.evaluate().map_err(|e| e.to_string())?;
                let joint = self.scene_joint(doc, &e).ok_or("Select a joint")?;
                if id.name == "MZ Add Part" || id.name == "MZ Add Profile" {
                    self.inspector.part = if self.scene.shift {
                        scene::add_rigid_profile(doc, &joint)?
                    } else {
                        scene::add_profile(doc, &joint, None)?
                    };
                    self.scene.tool = 3;
                    let op = scene::profiles(doc).unwrap().0.to_string();
                    self.scene_select(op);
                } else if id.name == "MZ Mirror Branch" {
                    let new = scene::mirror(doc, &joint)?;
                    self.inspector.joint = scene::armature(doc)
                        .unwrap()
                        .1
                        .iter()
                        .position(|j| j.id == new)
                        .unwrap();
                } else {
                    let mut removed_joints = std::collections::HashSet::new();
                    let mut removed_parts = std::collections::HashSet::new();
                    if self.scene.tool == 3 {
                        let parts = scene::parts_mut(doc)?;
                        if self.inspector.part >= parts.len() {
                            return Err("Select a part".into());
                        }
                        removed_parts.insert(parts.remove(self.inspector.part).id);
                        self.inspector.part = 0;
                    } else {
                        let defs = scene::armature_mut(doc)?;
                        if defs
                            .iter()
                            .find(|j| j.id == joint)
                            .is_none_or(|j| j.parent.is_none())
                        {
                            return Err("Keep the skeleton root".into());
                        }
                        removed_joints.insert(joint);
                        loop {
                            let n = removed_joints.len();
                            for j in defs.iter() {
                                if j.parent
                                    .as_ref()
                                    .is_some_and(|p| removed_joints.contains(p))
                                {
                                    removed_joints.insert(j.id.clone());
                                }
                            }
                            if n == removed_joints.len() {
                                break;
                            }
                        }
                        defs.retain(|j| !removed_joints.contains(&j.id));
                        for p in scene::parts_mut(doc)?.iter() {
                            if p.anchors.iter().any(|a| removed_joints.contains(&a.joint)) {
                                removed_parts.insert(p.id.clone());
                            }
                        }
                        scene::parts_mut(doc)?.retain(|p| !removed_parts.contains(&p.id));
                        self.inspector.joint = 0;
                    }
                    prune(&mut root, &removed_joints, &removed_parts);
                }
                self.scene_commit_root(root, project, ctx)?;
                self.inspector_dirty = true;
                return Ok(true);
            }
            TheEvent::ValueChanged(id, TheValue::Text(label))
                if id.name == "MZ Joint Name" || id.name == "MZ Profile Name" =>
            {
                let mut root = self.scene_root(project)?;
                let doc = root
                    .group_document_mut(&self.group_path)
                    .map_err(|e| e.to_string())?;
                if self.scene.tool == 3 {
                    let parts = scene::parts_mut(doc)?;
                    if let Some(p) = parts.get_mut(self.inspector.part) {
                        p.label = label.chars().take(64).collect();
                    }
                } else {
                    if let Some(j) = scene::armature_mut(doc)?.get_mut(self.inspector.joint) {
                        j.label = label.chars().take(64).collect();
                    }
                }
                self.scene_commit_root(root, project, ctx)?;
                return Ok(true);
            }
            TheEvent::RenderViewContext(id, p) if id.name == PREVIEW && self.scene.tool == 3 => {
                self.finish_drag(project, ctx, false);
                let doc = self.document.as_ref().ok_or("Create a model")?;
                let e = self.evaluation.as_ref().ok_or("No model")?;
                let hit = self.scene.hit(doc, e, &self.inspector, *p);
                let frame = self
                    .scene
                    .projection
                    .as_ref()
                    .ok_or("Preview is not ready")?;
                let rig = e.mesh.rig.as_ref().ok_or("Add a skeleton")?;
                let part = scene::profiles(doc)
                    .and_then(|(_, _, parts)| parts.get(self.inspector.part))
                    .ok_or("Select a part")?;
                let mut next = part.clone();
                if let Some((Handle::Profile(_, ring, _), _)) = hit {
                    if next.sections.len() <= 2 {
                        return Err("Keep at least two profile rings".into());
                    }
                    next.sections.remove(ring);
                    self.inspector.section = ring.min(next.sections.len() - 1);
                } else {
                    if next.sections.len() >= 32 {
                        return Err("A profile supports at most 32 rings".into());
                    }
                    let edge = next
                        .sections
                        .windows(2)
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            let distance = |s: &[denrim_materials::geometry::Section]| {
                                let center =
                                    std::array::from_fn(|k| (s[0].center[k] + s[1].center[k]) / 2.);
                                let q =
                                    frame.project(part.map(rig, center, [0., 1., 0.]).unwrap().0);
                                (q[0] - p.x as f32).hypot(q[1] - p.y as f32)
                            };
                            distance(a).total_cmp(&distance(b))
                        })
                        .map(|(i, _)| i)
                        .ok_or("Add two profile rings")?;
                    let a = &next.sections[edge];
                    let b = &next.sections[edge + 1];
                    let ring = denrim_materials::geometry::Section {
                        center: std::array::from_fn(|k| (a.center[k] + b.center[k]) / 2.),
                        radius: std::array::from_fn(|k| (a.radius[k] + b.radius[k]) / 2.),
                    };
                    next.sections.insert(edge + 1, ring);
                    self.inspector.section = edge + 1;
                }
                let mut root = self.scene_root(project)?;
                let d = root
                    .group_document_mut(&self.group_path)
                    .map_err(|e| e.to_string())?;
                scene::parts_mut(d)?[self.inspector.part] = next;
                self.scene_commit_root(root, project, ctx)?;
                self.inspector_dirty = true;
                return Ok(true);
            }
            TheEvent::RenderViewClicked(id, p) if id.name == PREVIEW && self.scene.tool > 0 => {
                if self.scene.alt && self.scene.tool == 2 {
                    return Ok(false);
                }
                self.finish_drag(project, ctx, false);
                let doc = self.document.as_ref().ok_or("Create a model")?;
                let e = self.evaluation.as_ref().ok_or("No model")?;
                let source = self
                    .selected
                    .and_then(|id| project.procedural_appearances.get(&id))
                    .cloned()
                    .ok_or("No source")?;
                if let Some((handle, point)) = self.scene.hit(doc, e, &self.inspector, *p) {
                    if self.scene.tool != 4 && self.playback.selected > 0 {
                        self.playback.rest();
                        self.scene.posed.clear();
                        self.preview_key.clear();
                        Self::status(ctx, "Rest pose selected for rig editing");
                        return Ok(true);
                    }
                    match &handle {
                        Handle::Joint(j) => {
                            self.inspector.joint = scene::armature(doc)
                                .and_then(|(_, defs)| defs.iter().position(|d| &d.id == j))
                                .or_else(|| {
                                    e.mesh
                                        .rig
                                        .as_ref()
                                        .and_then(|r| r.joints.iter().position(|d| &d.id == j))
                                })
                                .unwrap_or(0);
                            if self.scene.tool == 4 {
                                if let Some((row, _)) = self
                                    .playback
                                    .selected
                                    .checked_sub(1)
                                    .and_then(|i| self.playback.clips.get(i))
                                {
                                    if let Some(tile) = doc
                                        .rows
                                        .iter()
                                        .find(|r| &r.id == row)
                                        .and_then(|r| r.tiles.first())
                                    {
                                        if let Operation::Motion { tracks, .. } = &tile.op {
                                            self.inspector.track = tracks
                                                .iter()
                                                .position(|t| &t.joint == j)
                                                .unwrap_or(0);
                                        }
                                        self.scene.select_after_refresh = Some(tile.id.clone());
                                        self.selected_operation = Some(tile.id.clone());
                                    }
                                }
                            } else if let Some((id, _)) = scene::armature(doc) {
                                self.scene.select_after_refresh = Some(id.into());
                                self.selected_operation = Some(id.into());
                            }
                        }
                        Handle::Profile(part, ring, _) => {
                            self.inspector.part = *part;
                            self.inspector.section = *ring;
                        }
                        Handle::Grip(tile) => {
                            self.scene.select_after_refresh = Some(tile.clone());
                            self.selected_operation = Some(tile.clone());
                        }
                        Handle::Socket(row) => {
                            if let Some(t) = doc.rows.iter().find(|r| &r.id == row).and_then(|r| {
                                r.tiles
                                    .iter()
                                    .find(|t| matches!(t.op, Operation::Attach { .. }))
                            }) {
                                self.scene.select_after_refresh = Some(t.id.clone());
                                self.selected_operation = Some(t.id.clone());
                            }
                        }
                    }
                    self.inspector_dirty = true;
                    self.playback.playing = false;
                    self.scene.drag = Some(Drag {
                        handle,
                        start: *p,
                        source,
                        point,
                        frame: self
                            .scene
                            .projection
                            .clone()
                            .ok_or("Preview is not ready")?,
                    });
                    return Ok(true);
                }
                if self.scene.tool == 2 {
                    if self.playback.selected > 0 {
                        self.playback.rest();
                        self.scene.posed.clear();
                        self.preview_key.clear();
                        Self::status(ctx, "Rest pose selected for rig editing");
                        return Ok(true);
                    }
                    let frame = self
                        .scene
                        .projection
                        .as_ref()
                        .ok_or("Preview is not ready")?;
                    let joint = self.scene_joint(doc, e).ok_or("Select a parent joint")?;
                    let r = e.mesh.rig.as_ref().unwrap();
                    let parent = r.rest()[r.index(&joint).map_err(|e| e.to_string())?].translation;
                    let q = frame.project(parent);
                    let delta = frame.delta([p.x as f32 - q[0], p.y as f32 - q[1]]);
                    let target = std::array::from_fn(|k| parent[k] + delta[k]);
                    let mut root = self.scene_root(project)?;
                    let d = root
                        .group_document_mut(&self.group_path)
                        .map_err(|e| e.to_string())?;
                    let new = scene::add_joint(d, &joint, target)?;
                    self.inspector.joint = scene::armature(d)
                        .unwrap()
                        .1
                        .iter()
                        .position(|j| j.id == new)
                        .unwrap();
                    self.scene_commit_root(root, project, ctx)?;
                    self.inspector_dirty = true;
                    return Ok(true);
                }
                return Ok(false);
            }
            TheEvent::RenderViewDragged(id, p)
                if id.name == PREVIEW && self.scene.drag.is_some() =>
            {
                let drag = self.scene.drag.clone().unwrap();
                let mut root = Document::load(&drag.source).map_err(|e| e.to_string())?;
                let doc = root
                    .group_document_mut(&self.group_path)
                    .map_err(|e| e.to_string())?;
                let dp = [(p.x - drag.start.x) as f32, (p.y - drag.start.y) as f32];
                let delta = drag.frame.delta(dp);
                match drag.handle {
                    Handle::Joint(joint) => {
                        if self.scene.tool == 4 {
                            let row = self
                                .playback
                                .selected
                                .checked_sub(1)
                                .and_then(|i| self.playback.clips.get(i))
                                .ok_or("Create a motion clip")?
                                .0
                                .clone();
                            scene::motion_key(
                                doc,
                                &row,
                                &joint,
                                self.playback.phase,
                                if self.scene.alt {
                                    [0., 0., dp[0] * 0.5]
                                } else {
                                    [dp[1] * 0.5, dp[0] * 0.5, 0.]
                                },
                            )?;
                            if let Some(Operation::Motion { tracks, .. }) = doc
                                .rows
                                .iter()
                                .find(|r| r.id == row)
                                .and_then(|r| r.tiles.first())
                                .map(|t| &t.op)
                            {
                                if let Some(index) = tracks.iter().position(|t| t.joint == joint) {
                                    self.inspector.track = index;
                                    self.inspector.key = tracks[index]
                                        .keys
                                        .iter()
                                        .position(|k| (k.time - self.playback.phase).abs() < 0.0001)
                                        .unwrap_or(0);
                                }
                            }
                        } else {
                            authoring::set_world(
                                scene::armature_mut(doc)?,
                                &joint,
                                std::array::from_fn(|k| drag.point[k] + delta[k]),
                            )
                            .map_err(|e| e.to_string())?;
                        }
                    }
                    Handle::Profile(part, ring, axis) => {
                        let e = doc.evaluate().map_err(|e| e.to_string())?;
                        let rig = e.mesh.rig.as_ref().ok_or("No skeleton")?;
                        let parts = scene::parts_mut(doc)?;
                        let base = parts[part].clone();
                        let section = &base.sections[ring];
                        if let Some(axis) = axis {
                            let center = base
                                .map(rig, section.center, [0., 1., 0.])
                                .map_err(|e| e.to_string())?
                                .0;
                            let a = drag.frame.project(center);
                            let b = drag.frame.project(drag.point);
                            let vector = [b[0] - a[0], b[1] - a[1]];
                            let length = vector[0] * vector[0] + vector[1] * vector[1];
                            if length < 4. {
                                return Err("Use another camera angle to edit this radius".into());
                            }
                            parts[part].sections[ring].radius[axis] = (section.radius[axis]
                                * (1. + (dp[0] * vector[0] + dp[1] * vector[1]) / length))
                                .max(0.001);
                        } else {
                            let target = std::array::from_fn(|k| drag.point[k] + delta[k]);
                            parts[part].sections[ring].center =
                                inverse_profile(&base, rig, section.center, target)?;
                        }
                    }
                    Handle::Grip(tile) => {
                        if let Some(op) = doc
                            .rows
                            .iter_mut()
                            .flat_map(|r| &mut r.tiles)
                            .find(|t| t.id == tile)
                            .map(|t| &mut t.op)
                        {
                            match op {
                                Operation::Grip {
                                    translation,
                                    rotation,
                                } => {
                                    if self.scene.alt {
                                        rotation[0] += dp[1] * 0.5;
                                        rotation[1] += dp[0] * 0.5;
                                    } else {
                                        for k in 0..3 {
                                            translation[k] += delta[k];
                                        }
                                    }
                                }
                                Operation::ParticleEffect { effect } => {
                                    for k in 0..3 {
                                        effect.position[k] += delta[k];
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Handle::Socket(row) => {
                        let e = doc.evaluate().map_err(|e| e.to_string())?;
                        let a = e
                            .mesh
                            .attachments
                            .iter()
                            .find(|a| a.id == row)
                            .ok_or("No attachment")?;
                        let rest = e.mesh.rig.as_ref().unwrap().rest();
                        let local = rest[a.joint].inverse().vector(delta);
                        let op = &mut doc
                            .rows
                            .iter_mut()
                            .find(|r| r.id == row)
                            .unwrap()
                            .tiles
                            .iter_mut()
                            .find(|t| matches!(t.op, Operation::Attach { .. }))
                            .unwrap()
                            .op;
                        if let Operation::Attach {
                            translation,
                            rotation,
                            ..
                        } = op
                        {
                            if self.scene.alt {
                                rotation[0] += dp[1] * 0.5;
                                rotation[1] += dp[0] * 0.5;
                            } else {
                                for k in 0..3 {
                                    translation[k] += local[k];
                                }
                            }
                        }
                    }
                }
                let text = root.save().map_err(|e| e.to_string())?;
                project
                    .procedural_appearances
                    .insert(self.selected.unwrap(), text);
                self.local_edit = true;
                return Ok(true);
            }
            TheEvent::RenderViewUp(id, _) if id.name == PREVIEW && self.scene.drag.is_some() => {
                self.finish_scene_drag(project, ctx, false);
                return Ok(true);
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Escape))
                if self.scene.drag.is_some() =>
            {
                self.finish_scene_drag(project, ctx, true);
                return Ok(true);
            }
            _ => {}
        }
        Ok(false)
    }
    fn scene_add_motion(
        &mut self,
        project: &mut Project,
        ctx: &mut TheContext,
    ) -> Result<(), String> {
        let mut root = self.scene_root(project)?;
        if !self.playback.clips.is_empty() && self.group_path != self.playback.scope {
            self.group_path = self.playback.scope.clone();
            self.key = None;
        }
        let doc = root
            .group_document_mut(&self.group_path)
            .map_err(|e| e.to_string())?;
        let e = doc.evaluate().map_err(|e| e.to_string())?;
        let source = doc
            .rows
            .iter()
            .find(|r| e.rows.get(&r.id).and_then(Value::rig).is_some())
            .ok_or("Add a skeleton first")?
            .id
            .clone();
        let joint = e.rows[&source].rig().unwrap().joints[0].id.clone();
        let id = Uuid::new_v4().to_string();
        let tile = Uuid::new_v4().to_string();
        doc.rows.push(Row {
            id: id.clone(),
            label: format!(
                "Motion {}",
                doc.rows
                    .iter()
                    .filter(|r| r
                        .tiles
                        .iter()
                        .any(|t| matches!(t.op, Operation::Motion { .. })))
                    .count()
                    + 1
            ),
            tiles: vec![Tile {
                id: tile.clone(),
                op: Operation::Motion {
                    events: vec![],
                    rig: source,
                    duration: 1.,
                    looping: false,
                    tracks: vec![Track {
                        joint,
                        keys: vec![
                            Key {
                                visible: true,
                                time: 0.,
                                translation: [0.; 3],
                                rotation: [0.; 3],
                            },
                            Key {
                                visible: true,
                                time: 1.,
                                translation: [0.; 3],
                                rotation: [0.; 3],
                            },
                        ],
                    }],
                },
            }],
        });
        self.scene_commit_root(root, project, ctx)?;
        self.scene.pending_clip = Some(id);
        self.playback.phase = 0.;
        self.playback.playing = false;
        self.scene_select(tile);
        self.scene.tool = 4;
        self.preview_key.clear();
        Ok(())
    }
}
fn inverse_profile(
    part: &authoring::ProfilePart,
    rig: &denrim_materials::geometry::rig::Rig,
    mut p: [f32; 3],
    target: [f32; 3],
) -> Result<[f32; 3], String> {
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let dot = |a: [f32; 3], b: [f32; 3]| (0..3).map(|k| a[k] * b[k]).sum::<f32>();
    for _ in 0..5 {
        let q = part.map(rig, p, [0., 1., 0.]).map_err(|e| e.to_string())?.0;
        let error = std::array::from_fn(|k| target[k] - q[k]);
        let mut columns = [[0.; 3]; 3];
        for j in 0..3 {
            let mut s = p;
            s[j] += 0.0001;
            let v = part.map(rig, s, [0., 1., 0.]).map_err(|e| e.to_string())?.0;
            columns[j] = std::array::from_fn(|k| (v[k] - q[k]) / 0.0001);
        }
        let determinant = dot(columns[0], cross(columns[1], columns[2]));
        if determinant.abs() < 0.00001 {
            break;
        }
        let d = [
            dot(error, cross(columns[1], columns[2])) / determinant,
            dot(columns[0], cross(error, columns[2])) / determinant,
            dot(columns[0], cross(columns[1], error)) / determinant,
        ];
        for k in 0..3 {
            p[k] += d[k];
        }
    }
    Ok(p)
}
fn prune(
    doc: &mut Document,
    joints: &std::collections::HashSet<String>,
    parts: &std::collections::HashSet<String>,
) {
    for t in doc.rows.iter_mut().flat_map(|r| &mut r.tiles) {
        match &mut t.op {
            Operation::Group {
                document, controls, ..
            } => {
                for c in controls {
                    for m in &mut c.modifiers {
                        match m {
                            Modifier::Affine {
                                parts: p,
                                joints: j,
                                ..
                            } => {
                                p.retain(|p| !parts.contains(p));
                                j.retain(|j| !joints.contains(j));
                            }
                            Modifier::Palette { parts: p } => p.retain(|p| !parts.contains(p)),
                            _ => {}
                        }
                    }
                }
                prune(document, joints, parts);
            }
            Operation::Motion { tracks, .. } => tracks.retain(|t| !joints.contains(&t.joint)),
            Operation::Attach { joint, enabled, .. } if joints.contains(joint) => *enabled = false,
            _ => {}
        }
    }
}
