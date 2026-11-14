//! Owned CPU decode work. At most two jobs across controllers; no GPU/UI
//! handles, unbounded task queue, detached result publication or blocking join.
use super::*;
use std::sync::atomic::AtomicUsize;
#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(target_arch = "wasm32")]
pub(super) fn configure_browser_worker(url: &str) -> Result<(), String> {
    browser::configure(url)
}
#[cfg(target_arch = "wasm32")]
pub(super) fn browser_worker_main() -> Result<(), wasm_bindgen::JsValue> {
    browser::worker_main()
}
#[cfg(target_arch = "wasm32")]
pub(super) fn browser_worker_stats() -> BrowserCodecWorkerStats {
    browser::stats()
}

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
const WORKERS: usize = 2;
#[derive(Debug)]
pub(super) struct WorkerSlot;
impl WorkerSlot {
    pub(super) fn acquire() -> Option<Self> {
        ACTIVE
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < WORKERS).then_some(n + 1)
            })
            .ok()
            .map(|_| Self)
    }
}
impl Drop for WorkerSlot {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::Release);
    }
}
#[derive(Debug)]
pub(super) struct DecodeTask {
    // Inputs/output drop before their charge and worker slot on cancellation.
    input: [Option<String>; 5],
    pixels: DecodedBakeTile,
    charge: Charge,
    _slot: WorkerSlot,
    size: u32,
    stage: u8,
    valid: bool,
}
impl DecodeTask {
    pub(super) fn new(
        input: [Option<&str>; 5],
        size: u32,
        charge: Charge,
        slot: WorkerSlot,
    ) -> Self {
        Self {
            input: input.map(|s| s.map(str::to_owned)),
            pixels: Default::default(),
            charge,
            _slot: slot,
            size,
            stage: 0,
            valid: true,
        }
    }
    /// One original channel per cooperative step. Native workers call this
    /// off-thread; the single-thread browser yields back to RAF between steps.
    fn step(&mut self) -> bool {
        let encoded = self.input[self.stage as usize].as_deref();
        match self.stage {
            0 => {
                if let Some(color) = encoded.and_then(|s| decode_png(s, self.size)) {
                    self.pixels.color = color;
                } else {
                    self.valid = false;
                }
            }
            1 => {
                self.pixels.depth = encoded
                    .and_then(|s| decode_depth(s, self.size as usize * self.size as usize))
                    .unwrap_or_default()
            }
            2 => {
                self.pixels.albedo = encoded
                    .and_then(|s| decode_png(s, self.size))
                    .unwrap_or_default()
            }
            3 => {
                self.pixels.normal = encoded
                    .and_then(|s| decode_png(s, self.size))
                    .unwrap_or_default()
            }
            4 => {
                self.pixels.material = encoded
                    .and_then(|s| decode_png(s, self.size))
                    .unwrap_or_default()
            }
            _ => unreachable!(),
        }
        self.stage += 1;
        !self.valid || self.stage == 5
    }
    fn finish(mut self) -> Option<OrthographicBakeTileLease> {
        if !self.valid {
            return None;
        }
        // Free the encoded clone before releasing its reservation. The output
        // remains charged even if its controller/receiver has been retired.
        self.input = Default::default();
        let bytes = pixel_bytes(&self.pixels);
        if bytes > self.charge.bytes {
            return None;
        }
        self.charge.shrink(bytes);
        Some(OrthographicBakeTileLease(Arc::new(ChargedTile {
            pixels: self.pixels,
            _charge: self.charge,
        })))
    }
}

#[cfg(not(any(target_arch = "wasm32", feature = "single_thread")))]
#[derive(Debug)]
pub(super) struct DecodeWork {
    at: (i32, i32),
    result: std::sync::mpsc::Receiver<Option<OrthographicBakeTileLease>>,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}
#[cfg(not(any(target_arch = "wasm32", feature = "single_thread")))]
impl DecodeWork {
    pub(super) fn start(at: (i32, i32), mut task: DecodeTask) -> Result<Self, String> {
        let (send, result) = std::sync::mpsc::sync_channel(1);
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop = cancelled.clone();
        std::thread::Builder::new()
            .name("bake-source-decode".into())
            .spawn(move || {
                loop {
                    if stop.load(Ordering::Acquire) {
                        return;
                    }
                    if task.step() {
                        break;
                    }
                }
                if !stop.load(Ordering::Acquire) {
                    let _ = send.send(task.finish());
                }
            })
            .map_err(|e| format!("Cannot start bake source decode: {e}"))?;
        Ok(Self {
            at,
            result,
            cancelled,
        })
    }
    pub(super) fn at(&self) -> (i32, i32) {
        self.at
    }
    pub(super) fn poll(&mut self) -> Result<OrthographicBakeTilePoll, String> {
        use std::sync::mpsc::TryRecvError;
        match self.result.try_recv() {
            Ok(Some(lease)) => Ok(OrthographicBakeTilePoll::Ready(lease)),
            Ok(None) => Ok(OrthographicBakeTilePoll::Missing),
            Err(TryRecvError::Empty) => Ok(OrthographicBakeTilePoll::Pending),
            Err(TryRecvError::Disconnected) => Err("Bake source decode worker stopped.".into()),
        }
    }
}
#[cfg(not(any(target_arch = "wasm32", feature = "single_thread")))]
impl Drop for DecodeWork {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

#[cfg(any(target_arch = "wasm32", feature = "single_thread"))]
#[derive(Debug)]
pub(super) struct DecodeWork {
    at: (i32, i32),
    task: Option<DecodeTask>,
    #[cfg(target_arch = "wasm32")]
    ticket: Option<browser::Ticket>,
}
#[cfg(any(target_arch = "wasm32", feature = "single_thread"))]
impl DecodeWork {
    #[cfg(target_arch = "wasm32")]
    fn start_worker(&mut self) -> Result<(), String> {
        if self.ticket.is_none() {
            match browser::start(self.task.take().unwrap())? {
                browser::Start::Ticket(ticket) => self.ticket = Some(ticket),
                browser::Start::Waiting(task) => self.task = Some(task),
            }
        }
        Ok(())
    }
    pub(super) fn start(at: (i32, i32), task: DecodeTask) -> Result<Self, String> {
        #[cfg(target_arch = "wasm32")]
        if browser::configured() {
            let mut work = Self {
                at,
                task: Some(task),
                ticket: None,
            };
            work.start_worker()?;
            return Ok(work);
        }
        Ok(Self {
            at,
            task: Some(task),
            #[cfg(target_arch = "wasm32")]
            ticket: None,
        })
    }
    pub(super) fn at(&self) -> (i32, i32) {
        self.at
    }
    pub(super) fn poll(&mut self) -> Result<OrthographicBakeTilePoll, String> {
        #[cfg(target_arch = "wasm32")]
        if browser::configured() {
            self.start_worker()?;
            return self.ticket.as_mut().map_or(
                Ok(OrthographicBakeTilePoll::Pending),
                browser::Ticket::poll,
            );
        }
        if self.task.as_mut().unwrap().step() {
            Ok(self.task.take().unwrap().finish().map_or(
                OrthographicBakeTilePoll::Missing,
                OrthographicBakeTilePoll::Ready,
            ))
        } else {
            Ok(OrthographicBakeTilePoll::Pending)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cooperative_decode_yields_between_channels_and_retains_output_charge() {
        let color = BASE64.encode(encode_png(1, 1, &[3, 5, 7, 255]).unwrap());
        let depth = BASE64.encode(encode_depth(&[2.]));
        let input = [
            Some(color.as_str()),
            Some(depth.as_str()),
            Some(color.as_str()),
            Some(color.as_str()),
            Some(color.as_str()),
        ];
        let bytes = input.iter().flatten().map(|s| s.len() as u64).sum::<u64>() + 20;
        let budget = Arc::new(Budget(AtomicU64::new(bytes), AtomicU64::new(0)));
        let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let slot = loop {
            if let Some(slot) = WorkerSlot::acquire() {
                break slot;
            }
            assert!(std::time::Instant::now() < end);
            std::thread::yield_now();
        };
        let mut task = DecodeTask::new(
            input,
            1,
            Charge {
                bytes,
                budget: budget.clone(),
            },
            slot,
        );
        assert!(!task.step());
        assert_eq!(task.pixels.color, [3, 5, 7, 255]);
        assert!(task.pixels.depth.is_empty() && task.pixels.albedo.is_empty());
        assert!(!task.step());
        assert_eq!(task.pixels.depth, [2.]);
        assert!(task.pixels.albedo.is_empty());
        assert!(!task.step());
        assert!(!task.step());
        assert!(task.step());
        let lease = task.finish().unwrap();
        assert_eq!(budget.0.load(Ordering::Relaxed), 20);
        assert_eq!(lease.material, [3, 5, 7, 255]);
        drop(lease);
        assert_eq!(budget.0.load(Ordering::Relaxed), 0);
    }
}
