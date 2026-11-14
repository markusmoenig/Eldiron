//! A sample owns one private full-size working target. Regions partition that
//! target; no region is independently previewed, captured or committed.
use scenevm::paint_pages::PaintDispatchRegion;

pub(super) struct Regions {
    current: PaintDispatchRegion,
    waiting: Vec<PaintDispatchRegion>,
    #[cfg(test)]
    resolved: usize,
}
impl Regions {
    pub(super) fn new(size: u32) -> Self {
        Self {
            current: PaintDispatchRegion::full([size; 2]),
            waiting: Vec::new(),
            #[cfg(test)]
            resolved: 0,
        }
    }
    #[cfg(test)]
    pub(super) fn resolved(&self) -> usize {
        self.resolved
    }
    pub(super) fn current(&self) -> PaintDispatchRegion {
        self.current
    }
    pub(super) fn split(&mut self) -> bool {
        let Some([first, second]) = self.current.split() else {
            return false;
        };
        self.current = first;
        self.waiting.push(second);
        true
    }
    /// False means every pixel is resolved and the sample may be committed.
    pub(super) fn next(&mut self) -> bool {
        #[cfg(test)]
        {
            self.resolved += 1;
        }
        let Some(next) = self.waiting.pop() else {
            return false;
        };
        self.current = next;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn odd_regions_partition_every_absolute_pixel_once_without_overlap() {
        let mut regions = Regions {
            current: PaintDispatchRegion::full([13, 9]),
            waiting: Vec::new(),
            #[cfg(test)]
            resolved: 0,
        };
        let mut seen = [0u8; 13 * 9];
        loop {
            let region = regions.current();
            assert!(region.fits([13, 9]));
            if region.size[0] > 2 || region.size[1] > 2 {
                assert!(regions.split());
                continue;
            }
            for y in region.origin[1]..region.origin[1] + region.size[1] {
                for x in region.origin[0]..region.origin[0] + region.size[0] {
                    seen[(y * 13 + x) as usize] += 1;
                }
            }
            if !regions.next() {
                break;
            }
        }
        assert!(seen.iter().all(|count| *count == 1));
        assert!(PaintDispatchRegion::full([1, 1]).split().is_none());
        assert!(
            !PaintDispatchRegion {
                origin: [u32::MAX, 0],
                size: [2, 1]
            }
            .fits([13, 9])
        );
    }
}
