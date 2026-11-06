use std::rc::Rc;

use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Document, Element, Event, EventTarget, MouseEvent};

use super::{super::Controller, Phase};

struct Listener {
    target: EventTarget,
    name: &'static str,
    callback: Closure<dyn FnMut(Event)>,
}

impl Listener {
    fn new(
        target: EventTarget,
        name: &'static str,
        handler: impl FnMut(Event) + 'static,
    ) -> Result<Self, JsValue> {
        let callback = Closure::<dyn FnMut(Event)>::new(handler);
        target.add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())?;
        Ok(Self {
            target,
            name,
            callback,
        })
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.name, self.callback.as_ref().unchecked_ref());
    }
}

pub(super) struct Hooks {
    document: Document,
    canvas: Element,
    _listeners: Vec<Listener>,
}

impl Hooks {
    pub(super) fn install(controller: &Rc<Controller>) -> Option<Self> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let canvas = document.get_element_by_id("canvas")?;
        let weak = Rc::downgrade(controller);
        let changed = Listener::new(document.clone().into(), "pointerlockchange", move |_| {
            if let Some(controller) = weak.upgrade() {
                controller.pointer_changed();
            }
        })
        .ok()?;
        let weak = Rc::downgrade(controller);
        let error = Listener::new(document.clone().into(), "pointerlockerror", move |_| {
            if let Some(controller) = weak.upgrade()
                && matches!(controller.pointer.phase.get(), Phase::Pending(_))
            {
                controller.pointer.phase.set(Phase::Idle);
                controller.pointer.unavailable.set(true);
                controller.refresh();
            }
        })
        .ok()?;
        let weak = Rc::downgrade(controller);
        #[allow(clippy::cast_precision_loss)]
        let moved = Listener::new(canvas.clone().into(), "mousemove", move |event| {
            if let Some(controller) = weak.upgrade()
                && controller.pointer.active().is_some()
                && let Ok(event) = event.dyn_into::<MouseEvent>()
            {
                controller
                    .pointer
                    .set_keys(event.alt_key(), event.shift_key());
                controller.relative_motion([event.movement_x() as f32, event.movement_y() as f32]);
            }
        })
        .ok()?;
        let mut listeners = vec![changed, error, moved];
        for name in ["blur", "pagehide"] {
            let weak = Rc::downgrade(controller);
            listeners.push(
                Listener::new(window.clone().into(), name, move |_| {
                    if let Some(controller) = weak.upgrade() {
                        controller.cancel();
                    }
                })
                .ok()?,
            );
        }
        Some(Self {
            document,
            canvas,
            _listeners: listeners,
        })
    }

    pub(super) fn request(&self) -> bool {
        let Ok(request) = Reflect::get(&self.canvas, &JsValue::from_str("requestPointerLock"))
        else {
            return false;
        };
        let Ok(request) = request.dyn_into::<Function>() else {
            return false;
        };
        let Ok(result) = request.call0(&self.canvas) else {
            return false;
        };
        if let Ok(promise) = result.dyn_into::<Promise>() {
            // Browser events own the state transitions. Consume a rejected
            // promise as well so the fallback does not create a JS error.
            spawn_local(async move {
                let _ = JsFuture::from(promise).await;
            });
        }
        true
    }

    pub(super) fn release(&self) {
        if self.owns_lock() {
            self.document.exit_pointer_lock();
        }
    }

    fn owns_lock(&self) -> bool {
        self.document.pointer_lock_element().as_ref() == Some(&self.canvas)
    }
}

impl Controller {
    fn pointer_changed(&self) {
        let Some(web) = self.pointer.web.get() else {
            return;
        };
        match (self.pointer.phase.get(), web.owns_lock()) {
            (Phase::Pending(Some(id)), true) => self.pointer.phase.set(Phase::Locked(id)),
            (Phase::Pending(None) | Phase::Idle, true) => {
                self.pointer.phase.set(Phase::Releasing);
                web.release();
            }
            (Phase::Locked(_), false) => {
                self.pointer.phase.set(Phase::Idle);
                self.cancel();
                return;
            }
            (Phase::Pending(_) | Phase::Releasing, false) => self.pointer.phase.set(Phase::Idle),
            _ => {}
        }
        self.refresh();
    }
}
