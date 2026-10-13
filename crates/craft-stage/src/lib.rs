//! craft-stage: mark where a long operation is, so a stall can say where it stuck. **Ours, never
//! offered upstream** (part of the self-logging work; see craft-log).
//!
//! ```
//! let job = craft_stage::scope("select.subject");
//! job.stage("load");
//! job.stage_with("grabcut", &[("iteration", 3.0), ("pixels", 1.2e6)]);
//! // … the scope ends when `job` is dropped
//! ```
//!
//! No dependencies, and next to free when nothing listens: until a listener is registered with
//! [`set_hook`] (craft-log does that when a program installs its logger), [`scope`] is one atomic
//! load and returns an inert scope, and its calls do nothing. Tests and other embedders pay
//! nothing. No clock is read here, so it builds and runs anywhere the library does.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Receives scope events. Implemented by craft-log's stall watch.
pub trait Hook: Sync {
    /// A scope named `name` began; `id` is unique in the process.
    fn enter(&self, id: u64, name: &'static str);
    /// Scope `id` moved to stage `stage`; `numbers` are a few (name, value) facts about it.
    fn stage(&self, id: u64, stage: &'static str, numbers: &[(&'static str, f64)]);
    /// Scope `id` ended.
    fn leave(&self, id: u64);
}

static HOOK: OnceLock<&'static dyn Hook> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(1);

/// Register the listener. Only the first call takes effect; returns whether this one did.
pub fn set_hook(hook: &'static dyn Hook) -> bool {
    HOOK.set(hook).is_ok()
}

/// Whether a listener is registered.
pub fn listening() -> bool {
    HOOK.get().is_some()
}

/// A named piece of work in progress. Ends when dropped (also when unwinding from a panic).
#[must_use = "the scope ends when this value is dropped"]
pub struct Scope {
    id: u64,
    hook: Option<&'static dyn Hook>,
}

/// Begin a scope named `name` (a command or job name: `import.prproj`, `export`, `select.subject`).
pub fn scope(name: &'static str) -> Scope {
    let Some(hook) = HOOK.get().copied() else { return Scope { id: 0, hook: None } };
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    hook.enter(id, name);
    Scope { id, hook: Some(hook) }
}

impl Scope {
    /// The scope moves on to stage `name`.
    pub fn stage(&self, name: &'static str) {
        self.stage_with(name, &[]);
    }

    /// The scope moves on to stage `name`, with a few numbers that say how far or how big.
    pub fn stage_with(&self, name: &'static str, numbers: &[(&'static str, f64)]) {
        if let Some(h) = self.hook {
            h.stage(self.id, name, numbers);
        }
    }

    /// Whether anything is listening to this scope.
    pub fn is_active(&self) -> bool {
        self.hook.is_some()
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if let Some(h) = self.hook {
            h.leave(self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn without_a_listener_a_scope_does_nothing() {
        // the listener below is registered by the other test, possibly first: either way this
        // scope must work and never panic
        let s = scope("idle");
        s.stage("one");
        s.stage_with("two", &[("n", 1.0)]);
        drop(s);
    }

    struct Recorder(Mutex<Vec<String>>);
    impl Hook for Recorder {
        fn enter(&self, id: u64, name: &'static str) {
            self.0.lock().unwrap().push(format!("enter {id} {name}"));
        }
        fn stage(&self, id: u64, stage: &'static str, numbers: &[(&'static str, f64)]) {
            self.0.lock().unwrap().push(format!("stage {id} {stage} {numbers:?}"));
        }
        fn leave(&self, id: u64) {
            self.0.lock().unwrap().push(format!("leave {id}"));
        }
    }

    #[test]
    fn a_listener_hears_enter_stages_and_leave_in_order() {
        static REC: Recorder = Recorder(Mutex::new(Vec::new()));
        set_hook(&REC);
        assert!(listening());
        let id;
        {
            let s = scope("import");
            assert!(s.is_active());
            id = s.id;
            s.stage("parse");
            s.stage_with("relink", &[("clips", 12.0)]);
        }
        let got: Vec<String> = REC.0.lock().unwrap().iter().filter(|l| l.split(' ').nth(1) == Some(&id.to_string())).cloned().collect();
        assert_eq!(
            got,
            [format!("enter {id} import"), format!("stage {id} parse []"), format!("stage {id} relink [(\"clips\", 12.0)]"), format!("leave {id}")]
        );
        assert!(!set_hook(&REC), "only the first listener is taken");
    }
}
