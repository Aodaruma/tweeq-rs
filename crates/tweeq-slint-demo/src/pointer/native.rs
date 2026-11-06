use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use slint::{
    ComponentHandle, Window,
    winit_030::{
        CustomApplicationHandler, EventResult, WinitWindowAccessor,
        winit::{
            event::{DeviceEvent, DeviceId, WindowEvent},
            event_loop::ActiveEventLoop,
            window::{CursorGrabMode, Window as NativeWindow, WindowId},
        },
    },
};

use super::super::{Controller, Gallery};

pub(super) fn grab(window: &Gallery) -> bool {
    window
        .window()
        .with_winit_window(|native| {
            if native.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                return false;
            }
            native.set_cursor_visible(false);
            true
        })
        .unwrap_or(false)
}

pub(super) fn release(window: &Gallery) {
    window.window().with_winit_window(|native| {
        let _ = native.set_cursor_grab(CursorGrabMode::None);
        native.set_cursor_visible(true);
    });
}

#[derive(Default, Clone)]
pub(crate) struct NativeHandler(Rc<RefCell<Weak<Controller>>>);

impl NativeHandler {
    pub(crate) fn connect(&self, controller: &Rc<Controller>) {
        self.0.replace(Rc::downgrade(controller));
    }
}

impl CustomApplicationHandler for NativeHandler {
    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _native: Option<&NativeWindow>,
        slint_window: Option<&Window>,
        event: &WindowEvent,
    ) -> EventResult {
        let Some(controller) = self.0.borrow().upgrade() else {
            return EventResult::Propagate;
        };
        let Some(window) = controller.window.upgrade() else {
            return EventResult::Propagate;
        };
        if !slint_window.is_some_and(|event_window| std::ptr::eq(event_window, window.window())) {
            return EventResult::Propagate;
        }
        match event {
            WindowEvent::Focused(false) | WindowEvent::CloseRequested => controller.cancel(),
            WindowEvent::ModifiersChanged(modifiers) => {
                let keys = modifiers.state();
                controller
                    .pointer
                    .set_keys(keys.alt_key(), keys.shift_key());
            }
            _ => {}
        }
        EventResult::Propagate
    }

    #[allow(clippy::cast_possible_truncation)]
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) -> EventResult {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event
            && let Some(controller) = self.0.borrow().upgrade()
            && controller.pointer.active().is_some()
            && let Some(window) = controller.window.upgrade()
        {
            let scale = f64::from(window.window().scale_factor());
            controller.relative_motion([(dx / scale) as f32, (dy / scale) as f32]);
        }
        EventResult::Propagate
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, time::Duration};

    use slint::winit_030::winit::{
        event_loop::EventLoop, platform::windows::EventLoopBuilderExtWindows,
    };
    use tweeq_slint::ParamId;

    use crate::{TweeqAction, TweeqInput, connect};

    use super::*;

    #[test]
    #[ignore = "requires a Windows desktop for real cursor capture"]
    fn windows_capture_accepts_motion_beyond_the_control_and_releases() {
        let mut builder = EventLoop::with_user_event();
        builder.with_any_thread(true);
        let handler = NativeHandler::default();
        slint::BackendSelector::new()
            .backend_name("winit".into())
            .with_winit_event_loop_builder(builder)
            .with_winit_custom_application_handler(handler.clone())
            .select()
            .expect("Winit backend");
        let window = Gallery::new().expect("gallery");
        let controller = connect(&window);
        handler.connect(&controller);
        window.set_unbounded(true);
        window.show().expect("visible gallery");

        let result = Rc::new(Cell::new(None));
        let outcome = result.clone();
        slint::Timer::single_shot(Duration::from_millis(250), move || {
            let input = |action| TweeqInput {
                key: "2".into(),
                action,
                capture: true,
                width: 200.0,
                ..TweeqInput::default()
            };
            let source = ParamId::from_u64(2);
            let start = controller.input(&input(TweeqAction::BeginNumber));
            let grabbed = controller.pointer.active() == Some(source);
            controller.relative_motion([4500.0, 0.0]);
            let value = controller.state.borrow().editor.number(source);
            let finished = controller.input(&input(TweeqAction::Finish));
            let released = controller.pointer.active().is_none();

            controller.input(&input(TweeqAction::BeginNumber));
            controller.relative_motion([100.0, 0.0]);
            controller.cancel();
            let restored = controller.state.borrow().editor.number(source);
            outcome.set(Some((start, grabbed, value, finished, released, restored)));
            slint::quit_event_loop().expect("active loop");
        });
        slint::run_event_loop().expect("native event loop");
        assert_eq!(
            result.get(),
            Some((true, true, Some(4510.0), true, true, Some(4510.0)))
        );
    }
}
