//! Browser GPU scopes complete asynchronously. One initialization/compile may
//! be in flight. Addressed completions cannot publish against another source;
//! camera/time changes reuse compiled sources without parsing assets again.
use super::*;
use crate::gpu_task::Task;
use std::{cell::Cell, rc::Rc, sync::mpsc::TryRecvError};
pub(super) struct Source<'a> {
    pub key: String,
    pub address: String,
    pub spec: &'a Texture,
}
struct Completion(Rc<Cell<bool>>);
impl Drop for Completion {
    fn drop(&mut self) {
        self.0.set(true);
    }
}
struct Pending {
    key: String,
    address: String,
    receive: Option<Task<Result<MaterialInstance, String>>>,
    done: Rc<Cell<bool>>,
}
#[derive(Default)]
pub(super) struct Compiler {
    initializing: Option<Task<Result<Arc<MaterialZ>, String>>>,
    runtime_error: Option<String>,
    pending: Option<Pending>,
    failures: HashMap<String, String>,
    ready: HashMap<String, String>,
}
impl Compiler {
    pub fn prepare(
        &mut self,
        context: Arc<GpuContext>,
        runtime: &mut Option<Arc<MaterialZ>>,
        instances: &mut HashMap<String, MaterialInstance>,
        sources: Vec<Source<'_>>,
        palette: &ThePalette,
    ) -> Result<bool, String> {
        let active: HashSet<_> = sources.iter().map(|source| source.key.as_str()).collect();
        let addresses: HashSet<_> = sources
            .iter()
            .map(|source| source.address.as_str())
            .collect();
        instances.retain(|key, _| active.contains(key.as_str()));
        self.ready.retain(|key, _| active.contains(key.as_str()));
        self.failures
            .retain(|key, _| addresses.contains(key.as_str()));
        if let Some(pending) = &mut self.pending
            && !addresses.contains(pending.address.as_str())
        {
            // Drop obsolete results immediately, even if a cached image makes
            // the host idle. Keep only a completion flag until this task ends.
            pending.receive = None;
            pending.key.clear();
            pending.address.clear();
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.done.get())
        {
            let pending = self.pending.take().unwrap();
            if addresses.contains(pending.address.as_str())
                && let Some(receive) = pending.receive
            {
                let result = receive
                    .try_recv()
                    .unwrap_or_else(|_| Err("Geometry material compilation was retired".into()));
                match result {
                    Ok(instance) => {
                        self.ready.insert(pending.key.clone(), pending.address);
                        instances.insert(pending.key, instance);
                    }
                    Err(error) => {
                        self.failures.insert(pending.address, error);
                    }
                }
            }
        }
        if sources.is_empty() {
            // A fixed runtime can outlive sources. Drain only the already-
            // started initializer; source toggles never start abandoned jobs.
            if let Some(receive) = &self.initializing {
                match receive.try_recv() {
                    Ok(Ok(ready)) => {
                        *runtime = Some(ready);
                        self.initializing = None;
                    }
                    Ok(Err(error)) => {
                        self.runtime_error = Some(error);
                        self.initializing = None;
                    }
                    Err(TryRecvError::Disconnected) => {
                        self.initializing = None;
                    }
                    Err(TryRecvError::Empty) => {}
                }
            }
            return Ok(true);
        }
        if let Some(error) = &self.runtime_error {
            return Err(error.clone());
        }
        if runtime.is_none() {
            if let Some(receive) = &self.initializing {
                match receive.try_recv() {
                    Ok(Ok(ready)) => {
                        *runtime = Some(ready);
                        self.initializing = None;
                    }
                    Ok(Err(error)) => {
                        self.initializing = None;
                        self.runtime_error = Some(error.clone());
                        return Err(error);
                    }
                    Err(TryRecvError::Empty) => return Ok(false),
                    Err(TryRecvError::Disconnected) => {
                        let error = "Geometry material initialization was retired".to_string();
                        self.initializing = None;
                        self.runtime_error = Some(error.clone());
                        return Err(error);
                    }
                }
            } else {
                self.initializing = Some(Task::spawn(async move {
                    MaterialZ::new_with_context(context)
                        .await
                        .map(Arc::new)
                        .map_err(|e| e.to_string())
                }));
                return Ok(false);
            }
        }
        for source in sources {
            if let Some(error) = self.failures.get(&source.address) {
                return Err(error.clone());
            }
            if self.ready.get(&source.key) == Some(&source.address)
                && instances.contains_key(&source.key)
            {
                continue;
            }
            let asset = super::super::textures::asset(source.spec, palette, None)?;
            if instances
                .get_mut(&source.key)
                .is_some_and(|instance| instance.update_asset(&asset).is_ok())
            {
                self.ready.insert(source.key, source.address);
                continue;
            }
            if self.pending.is_some() {
                return Ok(false);
            }
            let runtime = runtime.as_ref().unwrap().clone();
            let finished = Rc::new(Cell::new(false));
            let completion = Completion(finished.clone());
            let receive = Task::spawn(async move {
                let _completion = completion;
                runtime
                    .compile(&asset, InstanceRecord::new("native-geometry-texture"))
                    .await
                    .map_err(|e| e.to_string())
            });
            self.pending = Some(Pending {
                key: source.key,
                address: source.address,
                receive: Some(receive),
                done: finished,
            });
            return Ok(false);
        }
        Ok(true)
    }
}
