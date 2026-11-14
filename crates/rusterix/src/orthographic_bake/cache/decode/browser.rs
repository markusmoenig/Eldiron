//! Main-thread worker ownership. Cache-facing tickets contain only Send CPU
//! state; browser handles stay in this realm's bounded two-slot pool.
use super::*;
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::atomic::{AtomicBool, AtomicU64},
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

const VERSION: f64 = 1.;
static NEXT: AtomicU64 = AtomicU64::new(1);
#[derive(Debug, Default)]
struct Reply {
    result: Mutex<Option<Result<OrthographicBakeTilePoll, String>>>,
    cancelled: AtomicBool,
    finished: AtomicBool,
}
#[derive(Debug)]
pub(super) struct Ticket {
    reply: Arc<Reply>,
}
impl Ticket {
    pub(super) fn poll(&mut self) -> Result<OrthographicBakeTilePoll, String> {
        self.reply
            .result
            .lock()
            .unwrap()
            .take()
            .unwrap_or(Ok(OrthographicBakeTilePoll::Pending))
    }
}
impl Drop for Ticket {
    fn drop(&mut self) {
        if !self.reply.finished.load(Ordering::Acquire)
            && !self.reply.cancelled.swap(true, Ordering::AcqRel)
        {
            POOL.with(|pool| pool.borrow_mut().cancelled += 1);
        }
        // The live slot retains task/charge/WorkerSlot until result or error.
        // Requesting termination alone must not release active decode capacity.
    }
}
struct Job {
    _bridge: BridgeCharge,
    id: u64,
    task: DecodeTask,
    reply: Arc<Reply>,
}
struct Slot {
    worker: Worker,
    ready: bool,
    failed: bool,
    job: Option<Job>,
    message: Option<Closure<dyn FnMut(MessageEvent)>>,
    error: Option<Closure<dyn FnMut(web_sys::Event)>>,
}
impl Drop for Slot {
    fn drop(&mut self) {
        self.worker.set_onmessage(None);
        self.worker.set_onerror(None);
        self.worker.terminate();
    }
}
#[derive(Default)]
struct Pool {
    url: Option<String>,
    slots: Vec<Rc<RefCell<Slot>>>,
    started: u64,
    completed: u64,
    cancelled: u64,
    failures: u64,
}
thread_local! { static POOL: RefCell<Pool> = RefCell::new(Pool::default()); }

pub(super) fn configure(url: &str) -> Result<(), String> {
    if url.trim().is_empty() {
        return Err("Empty browser codec worker URL".into());
    }
    POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        if pool.url.as_deref() == Some(url) {
            return Ok(());
        }
        if !pool.slots.is_empty() {
            return Err("Browser codec workers are already configured".into());
        }
        pool.url = Some(url.into());
        Ok(())
    })
}
pub(super) fn configured() -> bool {
    POOL.with(|pool| pool.borrow().url.is_some())
}
pub(super) fn stats() -> BrowserCodecWorkerStats {
    POOL.with(|pool| {
        let pool = pool.borrow();
        BrowserCodecWorkerStats {
            configured: pool.url.is_some(),
            workers: pool.slots.len(),
            active_jobs: ACTIVE.load(Ordering::Acquire),
            requests: pool.started,
            completed: pool.completed,
            cancelled: pool.cancelled,
            failures: pool.failures,
            job_bytes: pool
                .slots
                .iter()
                .filter_map(|slot| slot.borrow().job.as_ref().map(|job| job.task.charge.bytes))
                .sum(),
            bridge_bytes: BRIDGE_BYTES.load(Ordering::Acquire),
            bridge_limit: BRIDGE_LIMIT,
        }
    })
}
fn set(object: &JsValue, key: &str, value: &JsValue) -> Result<(), String> {
    Reflect::set(object, &key.into(), value)
        .map(|_| ())
        .map_err(|e| format!("Codec worker protocol: {e:?}"))
}
fn get(object: &JsValue, key: &str) -> Result<JsValue, String> {
    Reflect::get(object, &key.into()).map_err(|e| format!("Codec worker protocol: {e:?}"))
}
fn object(kind: &str, id: &str) -> Result<Object, String> {
    let object = Object::new();
    set(&object, "version", &VERSION.into())?;
    set(&object, "kind", &kind.into())?;
    set(&object, "id", &id.into())?;
    Ok(object)
}
fn send(slot: &mut Slot) -> Result<(), String> {
    if !slot.ready {
        return Ok(());
    }
    if slot
        .job
        .as_ref()
        .is_some_and(|job| job.reply.cancelled.load(Ordering::Acquire))
    {
        let job = slot.job.take().unwrap();
        job.reply.finished.store(true, Ordering::Release);
        // Readiness confirms this worker is idle; this cancelled request was
        // never sent. It has no off-thread work or transfer ownership to await.
        return Ok(());
    }
    let Some(job) = &mut slot.job else {
        return Ok(());
    };
    let request = object("decode", &job.id.to_string())?;
    set(&request, "size", &job.task.size.into())?;
    let channels = Array::new();
    let transfer = Array::new();
    for input in &mut job.task.input {
        if let Some(input) = input.take() {
            let bytes = Uint8Array::from(input.as_bytes());
            transfer.push(&bytes.buffer());
            channels.push(&bytes);
        } else {
            channels.push(&JsValue::NULL);
        }
    }
    set(&request, "channels", &channels)?;
    slot.worker
        .post_message_with_transfer(&request, &transfer)
        .map_err(|e| format!("Cannot send original codec inputs: {e:?}"))
}
fn fail(slot: &mut Slot, error: String) {
    slot.failed = true;
    slot.worker.terminate();
    if let Some(job) = slot.job.take() {
        job.reply.finished.store(true, Ordering::Release);
        *job.reply.result.lock().unwrap() = Some(Err(error));
        // An error event is actual worker retirement, not controller cancellation.
    }
    POOL.with(|pool| pool.borrow_mut().failures += 1);
}
fn receive(weak: &Weak<RefCell<Slot>>, event: MessageEvent) {
    let Some(slot) = weak.upgrade() else {
        return;
    };
    let mut slot = slot.borrow_mut();
    let data = event.data();
    let result = (|| -> Result<(), String> {
        if get(&data, "version")?.as_f64() != Some(VERSION) {
            return Err("Codec worker version mismatch".into());
        }
        let kind = get(&data, "kind")?
            .as_string()
            .ok_or("Missing codec worker message kind")?;
        if kind == "ready" {
            if slot.ready {
                return Err("Duplicate codec worker readiness".into());
            }
            if get(&data, "window")?.as_bool() != Some(false) {
                return Err("Codec worker initialized in a Window realm".into());
            }
            slot.ready = true;
            return send(&mut slot);
        }
        let Some(job) = slot.job.as_ref() else {
            return Ok(());
        };
        if get(&data, "id")?.as_string().as_deref() != Some(&job.id.to_string()) {
            return Ok(());
        }
        if kind != "complete" {
            return Err("Unexpected codec worker reply".into());
        }
        let mut job = slot.job.take().unwrap();
        job.reply.finished.store(true, Ordering::Release);
        if !job.reply.cancelled.load(Ordering::Acquire) {
            let decoded = read_output(&data, job.task.size);
            let result = decoded.map(|pixels| {
                job.task.valid = pixels.is_some();
                job.task.pixels = pixels.unwrap_or_default();
                job.task.finish().map_or(
                    OrthographicBakeTilePoll::Missing,
                    OrthographicBakeTilePoll::Ready,
                )
            });
            if result.is_err() {
                POOL.with(|pool| pool.borrow_mut().failures += 1);
            }
            *job.reply.result.lock().unwrap() = Some(result);
        }
        POOL.with(|pool| pool.borrow_mut().completed += 1);
        Ok(())
    })();
    if let Err(error) = result {
        fail(&mut slot, error);
    }
}
fn create(url: &str) -> Result<Rc<RefCell<Slot>>, String> {
    let options = WorkerOptions::new();
    options.set_type(WorkerType::Module);
    let worker = Worker::new_with_options(url, &options)
        .map_err(|e| format!("Cannot create codec worker: {e:?}"))?;
    let slot = Rc::new(RefCell::new(Slot {
        worker,
        ready: false,
        failed: false,
        job: None,
        message: None,
        error: None,
    }));
    let weak = Rc::downgrade(&slot);
    let message = Closure::new(move |event| receive(&weak, event));
    let weak = Rc::downgrade(&slot);
    let error = Closure::new(move |_: web_sys::Event| {
        if let Some(slot) = weak.upgrade() {
            fail(
                &mut slot.borrow_mut(),
                "Browser codec worker failed to load or execute".into(),
            );
        }
    });
    {
        let mut state = slot.borrow_mut();
        state
            .worker
            .set_onmessage(Some(message.as_ref().unchecked_ref()));
        state
            .worker
            .set_onerror(Some(error.as_ref().unchecked_ref()));
        state.message = Some(message);
        state.error = Some(error);
    }
    Ok(slot)
}
// Covers transferable/JS/Wasm comparison copies beyond the task's original
// encoded+maximum-decoded reservation. Pending requests retain their existing
// charge/worker slot; no unbounded transport queue or hidden UI-thread decode.
const BRIDGE_LIMIT: u64 = 64 * 1024 * 1024;
static BRIDGE_BYTES: AtomicU64 = AtomicU64::new(0);
struct BridgeCharge(u64);
impl BridgeCharge {
    fn acquire(bytes: u64) -> Result<Option<Self>, String> {
        if bytes > BRIDGE_LIMIT {
            return Err("Codec transport exceeds bounded budget".into());
        }
        Ok(BRIDGE_BYTES
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                n.checked_add(bytes).filter(|n| *n <= BRIDGE_LIMIT)
            })
            .ok()
            .map(|_| Self(bytes)))
    }
}
impl Drop for BridgeCharge {
    fn drop(&mut self) {
        BRIDGE_BYTES.fetch_sub(self.0, Ordering::Release);
    }
}
pub(super) enum Start {
    Ticket(Ticket),
    Waiting(DecodeTask),
}
pub(super) fn start(task: DecodeTask) -> Result<Start, String> {
    let Some(bridge) = BridgeCharge::acquire(
        task.charge
            .bytes
            .checked_mul(2)
            .ok_or("Codec transport size overflow")?,
    )?
    else {
        return Ok(Start::Waiting(task));
    };
    let id = NEXT
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| "Codec job identity exhausted")?;
    let reply = Arc::new(Reply::default());
    let slot = POOL.with(|pool| -> Result<_, String> {
        let mut pool = pool.borrow_mut();
        pool.slots.retain(|slot| !slot.borrow().failed);
        let slot = if let Some(slot) = pool.slots.iter().find(|slot| slot.borrow().job.is_none()) {
            slot.clone()
        } else {
            if pool.slots.len() >= WORKERS {
                return Err("Codec worker slots are occupied".into());
            }
            let slot = create(pool.url.as_deref().ok_or("Codec worker URL unconfigured")?)?;
            pool.slots.push(slot.clone());
            slot
        };
        slot.borrow_mut().job = Some(Job {
            _bridge: bridge,
            id,
            task,
            reply: reply.clone(),
        });
        pool.started += 1;
        Ok(slot)
    })?;
    let sent = send(&mut slot.borrow_mut());
    if let Err(error) = sent {
        fail(&mut slot.borrow_mut(), error);
    }
    Ok(Start::Ticket(Ticket { reply }))
}
fn read_output(data: &JsValue, size: u32) -> Result<Option<DecodedBakeTile>, String> {
    match get(data, "valid")?.as_bool() {
        Some(false) => return Ok(None),
        Some(true) => (),
        None => return Err("Invalid codec output validity flag".into()),
    }
    let max = size
        .checked_mul(size)
        .and_then(|n| n.checked_mul(4))
        .ok_or("Invalid codec output dimensions")?;
    let channels = get(data, "channels")?
        .dyn_into::<Array>()
        .map_err(|_| "Missing codec output channels")?;
    if channels.length() != 5 {
        return Err("Invalid codec output channel count".into());
    }
    let mut output = Vec::new();
    for index in 0..5 {
        let bytes = channels
            .get(index)
            .dyn_into::<Uint8Array>()
            .map_err(|_| "Invalid codec channel storage")?;
        if bytes.length() != max && !(index != 0 && bytes.length() == 0) {
            return Err("Invalid codec output channel dimensions".into());
        }
        output.push(bytes.to_vec());
    }
    let mut output = output.into_iter();
    Ok(Some(DecodedBakeTile {
        color: output.next().unwrap(),
        depth: output
            .next()
            .unwrap()
            .chunks_exact(4)
            .map(|p| f32::from_le_bytes(p.try_into().unwrap()))
            .collect(),
        albedo: output.next().unwrap(),
        normal: output.next().unwrap(),
        material: output.next().unwrap(),
    }))
}

/// Dedicated-worker entry. This module uses no Window, GPU, UI or session setup.
pub(super) fn worker_main() -> Result<(), JsValue> {
    let scope = js_sys::global().dyn_into::<web_sys::DedicatedWorkerGlobalScope>()?;
    let output = scope.clone();
    let listener = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let data = event.data();
        let result = (|| -> Result<(), String> {
            if get(&data, "version")?.as_f64() != Some(VERSION)
                || get(&data, "kind")?.as_string().as_deref() != Some("decode")
            {
                return Err("Invalid codec request version/kind".into());
            }
            let id = get(&data, "id")?
                .as_string()
                .ok_or("Missing codec request identity")?;
            let size = get(&data, "size")?
                .as_f64()
                .filter(|n| n.is_finite() && n.fract() == 0. && *n > 0. && *n <= 4096.)
                .ok_or("Invalid codec request size")? as u32;
            let channels = get(&data, "channels")?
                .dyn_into::<Array>()
                .map_err(|_| "Invalid codec inputs")?;
            if channels.length() != 5 {
                return Err("Invalid codec input count".into());
            }
            let mut charge = u64::from(size) * u64::from(size) * 20;
            let mut inputs: [Option<String>; 5] = Default::default();
            for (index, input) in inputs.iter_mut().enumerate() {
                let value = channels.get(index as u32);
                if value.is_null() {
                    continue;
                }
                let bytes = value
                    .dyn_into::<Uint8Array>()
                    .map_err(|_| "Invalid codec input storage")?;
                charge = charge
                    .checked_add(u64::from(bytes.length()))
                    .ok_or("Codec input overflow")?;
                if charge > DECODE_BUDGET {
                    return Err("Codec request exceeds bounded input/output budget".into());
                }
                *input =
                    Some(String::from_utf8(bytes.to_vec()).map_err(|_| "Invalid base64 text")?);
            }
            if charge > DECODE_BUDGET {
                return Err("Codec dimensions exceed decoded budget".into());
            }
            let pixels = inputs[0].take().and_then(|color| {
                decode_tile(
                    &OrthographicBakeTile {
                        x: 0,
                        y: 0,
                        color_png_base64: color.into(),
                        depth_base64: inputs[1].take().map(Into::into),
                        albedo_png_base64: inputs[2].take().map(Into::into),
                        normal_png_base64: inputs[3].take().map(Into::into),
                        material_png_base64: inputs[4].take().map(Into::into),
                        color_png_path: None,
                        depth_path: None,
                        albedo_png_path: None,
                        normal_png_path: None,
                        material_png_path: None,
                    },
                    size,
                )
            });
            let reply = object("complete", &id)?;
            set(&reply, "valid", &pixels.is_some().into())?;
            let transfer = Array::new();
            let channels = Array::new();
            if let Some(pixels) = pixels {
                let depth = pixels
                    .depth
                    .iter()
                    .flat_map(|n| n.to_le_bytes())
                    .collect::<Vec<_>>();
                for bytes in [
                    pixels.color,
                    depth,
                    pixels.albedo,
                    pixels.normal,
                    pixels.material,
                ] {
                    let channel = Uint8Array::from(bytes.as_slice());
                    transfer.push(&channel.buffer());
                    channels.push(&channel);
                }
            }
            set(&reply, "channels", &channels)?;
            output
                .post_message_with_transfer(&reply, &transfer)
                .map_err(|e| format!("Cannot publish codec output: {e:?}"))
        })();
        if let Err(error) = result {
            wasm_bindgen::throw_str(&error);
        }
    });
    scope.set_onmessage(Some(listener.as_ref().unchecked_ref()));
    listener.forget();
    let ready = object("ready", "").map_err(|e| JsValue::from_str(&e))?;
    set(&ready, "window", &false.into()).map_err(|e| JsValue::from_str(&e))?;
    scope.post_message(&ready)
}
