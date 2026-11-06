use std::cell::Cell;

use tweeq_slint::{GestureModifiers, ParamId};

use super::Gallery;

#[cfg(windows)]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(windows)]
pub(super) use native::NativeHandler;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    #[cfg(target_arch = "wasm32")]
    Pending(Option<ParamId>),
    Locked(ParamId),
    #[cfg(target_arch = "wasm32")]
    Releasing,
}

#[derive(Default)]
pub(super) struct Capture {
    phase: Cell<Phase>,
    modifiers: Cell<GestureModifiers>,
    unavailable: Cell<bool>,
    #[cfg(target_arch = "wasm32")]
    web: std::cell::OnceCell<web::Hooks>,
}

impl Capture {
    pub(super) fn active(&self) -> Option<ParamId> {
        if let Phase::Locked(id) = self.phase.get() {
            Some(id)
        } else {
            None
        }
    }

    pub(super) fn targets(&self, id: ParamId) -> bool {
        match self.phase.get() {
            Phase::Locked(key) => key == id,
            #[cfg(target_arch = "wasm32")]
            Phase::Pending(Some(key)) => key == id,
            _ => false,
        }
    }

    pub(super) fn request(&self, window: &Gallery, id: ParamId, modifiers: GestureModifiers) {
        if self.phase.get() != Phase::Idle {
            return;
        }
        self.modifiers.set(modifiers);
        self.unavailable.set(false);
        #[cfg(windows)]
        if native::grab(window) {
            self.phase.set(Phase::Locked(id));
            return;
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(web) = self.web.get() {
            self.phase.set(Phase::Pending(Some(id)));
            if web.request() {
                return;
            }
            self.phase.set(Phase::Idle);
        }
        let _ = (window, id);
        self.unavailable.set(true);
    }

    pub(super) fn release(&self, window: &Gallery) {
        #[cfg(windows)]
        if matches!(self.phase.replace(Phase::Idle), Phase::Locked(_)) {
            native::release(window);
        }
        #[cfg(target_arch = "wasm32")]
        match self.phase.get() {
            Phase::Pending(Some(_)) => self.phase.set(Phase::Pending(None)),
            Phase::Locked(_) => {
                self.phase.set(Phase::Releasing);
                if let Some(web) = self.web.get() {
                    web.release();
                }
            }
            _ => {}
        }
        let _ = window;
        self.unavailable.set(false);
    }

    pub(super) fn status(&self) -> &'static str {
        match self.phase.get() {
            Phase::Locked(_) => "Pointer locked",
            #[cfg(target_arch = "wasm32")]
            Phase::Pending(Some(_)) => "Requesting pointer lock",
            _ if self.unavailable.get() => "Pointer lock unavailable",
            _ => "",
        }
    }

    pub(super) fn set_snap(&self, snap: bool) {
        self.modifiers.set(GestureModifiers {
            snap,
            ..self.modifiers.get()
        });
    }

    #[cfg(any(windows, target_arch = "wasm32"))]
    fn set_keys(&self, fine: bool, fast: bool) {
        self.modifiers.set(GestureModifiers {
            fine,
            fast,
            ..self.modifiers.get()
        });
    }

    pub(super) fn modifiers(&self) -> GestureModifiers {
        self.modifiers.get()
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn install_web(&self, controller: &std::rc::Rc<super::Controller>) {
        if let Some(hooks) = web::Hooks::install(controller) {
            let _ = self.web.set(hooks);
        }
    }
}
