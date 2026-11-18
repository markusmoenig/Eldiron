//! CPU remote player. A host fetches missing content hints over HTTPS before
//! applying their frame, and drives this clock independently from the server.
use crate::*;
use eldiron_runtime::{MotionClient, MotionSettings};
use std::collections::HashMap;
pub struct RemoteClient {
    pub motion: MotionClient,
    pub assets: rusterix::Assets,
    installed: HashMap<RegionContentRef, (String, u64)>,
    resync_requested: bool,
}
impl RemoteClient {
    pub fn new(frame: &ClientFrame) -> Self {
        Self {
            motion: MotionClient::new(frame.world, frame.connection, MotionSettings::default()),
            assets: Default::default(),
            installed: Default::default(),
            resync_requested: false,
        }
    }
    pub fn needs_content(&self, hint: &ContentHint) -> bool {
        self.installed
            .get(&hint.reference)
            .is_none_or(|(key, stamp)| key != &hint.key || *stamp != hint.physics_revision)
    }
    pub fn install(&mut self, hint: &ContentHint, content: PhysicsContent) -> Result<(), String> {
        if content.reference != hint.reference || content.physics_revision != hint.physics_revision
        {
            return Err("Content reference mismatch".into());
        }
        let mut map = rusterix::Map::default();
        map.id = content.reference.map;
        map.changed = content.reference.map_revision;
        map.name = content.name;
        content.geometry.apply(&mut map);
        self.motion
            .install_content(content.reference, &map)
            .map_err(|e| e.to_string())?;
        self.motion.install_authoritative_collision(
            content.reference,
            content.mini,
            content.collision,
        );
        self.installed
            .insert(hint.reference, (hint.key.clone(), hint.physics_revision));
        Ok(())
    }
    pub fn receive(&mut self, frame: &ClientFrame) -> Result<(), String> {
        self.motion
            .receive(frame, &self.assets)
            .map_err(|e| e.to_string())?;
        if let eldiron_runtime::replication::WorldUpdate::Baseline(regions) = &frame.update {
            self.resync_requested = false;
            self.installed
                .retain(|key, _| regions.iter().any(|r| r.content == *key));
        }
        Ok(())
    }
    pub fn advance(&mut self, elapsed: std::time::Duration) -> ClientMessage {
        self.motion.advance(elapsed, &self.assets);
        if self.motion.needs_baseline() {
            if !self.resync_requested {
                self.resync_requested = true;
                return ClientMessage::Resync;
            }
            return ClientMessage::Inputs { frames: vec![] };
        }
        ClientMessage::Inputs {
            frames: self.motion.pending_inputs().into_iter().take(32).collect(),
        }
    }
}
pub fn content_url(endpoint: &str, key: &str) -> Result<String, String> {
    if key.len() != 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("Invalid content key".into());
    }
    let (scheme, rest) = if let Some(rest) = endpoint.strip_prefix("wss://") {
        ("https", rest)
    } else if let Some(rest) = endpoint.strip_prefix("ws://") {
        ("http", rest)
    } else {
        return Err("Invalid endpoint".into());
    };
    let authority = rest.split('/').next().unwrap_or("");
    if authority.is_empty() || authority.contains(['@', '?', '#']) {
        return Err("Invalid endpoint".into());
    }
    Ok(format!("{scheme}://{authority}/content/{key}"))
}
pub fn verify_content(hint: &ContentHint, bytes: &[u8]) -> Result<PhysicsContent, String> {
    use sha2::{Digest, Sha256};
    if format!("{:x}", Sha256::digest(bytes)) != hint.key {
        return Err("Content digest mismatch".into());
    }
    let content = codec::decode_content(bytes).map_err(|e| e.to_string())?;
    if content.reference != hint.reference || content.physics_revision != hint.physics_revision {
        return Err("Content reference mismatch".into());
    }
    if content.geometry.id != content.reference.map
        || content.geometry.revision != content.reference.map_revision
    {
        return Err("Geometry reference mismatch".into());
    }
    content.geometry.validate()?;
    Ok(content)
}
