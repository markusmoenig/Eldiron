//! Adds immutable release visuals to the existing bounded ordered content lane.
use crate::{
    public_assets::{PublicRegion, RegionAssets, RegionTask},
    *,
};
use std::collections::BTreeSet;
pub enum AssetDelivery {
    Region(PublicRegion),
    Content(ContentDelivery),
}
#[derive(Default)]
pub struct PublicAssetGate {
    assets: Option<RegionAssets>,
    loaded: BTreeSet<theframework_core::Uuid>,
    pending: Option<ContentDelivery>,
    task: Option<(theframework_core::Uuid, RegionTask)>,
}
impl PublicAssetGate {
    pub fn new(assets: Option<RegionAssets>) -> Self {
        Self {
            assets,
            ..Self::default()
        }
    }
    pub fn poll(
        &mut self,
        stream: &mut ContentStream,
        endpoint: &str,
    ) -> Result<Option<AssetDelivery>, String> {
        if let Some((map, task)) = &mut self.task {
            let Some(region) = task.poll() else {
                return Ok(None);
            };
            let region = region?;
            self.loaded.insert(*map);
            self.task = None;
            return Ok(Some(AssetDelivery::Region(region)));
        }
        if let Some(content) = self.pending.take() {
            return Ok(Some(AssetDelivery::Content(content)));
        }
        let Some(content) = stream.poll(endpoint)? else {
            return Ok(None);
        };
        if let ContentDelivery::Content(hint, _) = &content {
            if let Some(assets) = &self.assets {
                let map = hint.reference.map;
                if !self.loaded.contains(&map) {
                    let reference = assets
                        .regions
                        .iter()
                        .find(|r| r.map == Some(map))
                        .ok_or("Authoritative region is missing from the public release")?;
                    self.task = Some((
                        map,
                        RegionTask::start(assets.source.clone(), reference.clone())?,
                    ));
                    self.pending = Some(content);
                    return Ok(None);
                }
            }
        }
        Ok(Some(AssetDelivery::Content(content)))
    }
}
