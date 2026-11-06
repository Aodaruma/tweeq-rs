use std::{cell::RefCell, rc::Rc};

use slint::{Color, ComponentHandle};
use tweeq_slint::{Editor, GestureModifiers, NumberOptions, ParamId, ScrubStart};

slint::include_modules!();

const ID: ParamId = ParamId::from_u64(1);
const ANGLE: ParamId = ParamId::from_u64(2);
const COLOR: ParamId = ParamId::from_u64(3);

fn view(editor: &Editor, id: ParamId, key: &str) -> TweeqNumberView {
    let data = editor.number_view(id).expect("registered number");
    TweeqNumberView {
        key: key.into(),
        display: data.display.into(),
        edit_text: data.edit_text.into(),
        fraction: data.fraction,
        bounded: data.bounded,
        selected: data.selected,
    }
}

fn refresh(window: &Consumer, state: &Editor) {
    window.set_model(view(state, ID, "sample"));
    window.set_angle_model(view(state, ANGLE, "angle"));
    window.set_degrees(state.number(ANGLE).unwrap_or_default().rem_euclid(360.0) as f32);
    let color = state.color_view(COLOR).expect("registered color");
    let [red, green, blue, alpha] = color.rgba;
    let [hue, saturation, value, _] = color.hsva;
    window.set_color_model(TweeqColorView {
        key: "color".into(),
        rgba: Color::from_argb_f32(alpha, red, green, blue),
        pure_hue: Color::from_rgb_u8(255, 0, 0),
        hex: color.hex.into(),
        red,
        green,
        blue,
        alpha,
        hue,
        saturation,
        value,
        selected: color.selected,
    });
}

fn main() -> Result<(), slint::PlatformError> {
    let window = Consumer::new()?;
    let editor = Rc::new(RefCell::new(Editor::default()));
    editor
        .borrow_mut()
        .insert_number(ID, 16_777_217.0, NumberOptions::default());
    editor
        .borrow_mut()
        .insert_number(ANGLE, 0.0, NumberOptions::default());
    editor
        .borrow_mut()
        .insert_color(COLOR, [1.0, 0.0, 0.0, 1.0]);
    refresh(&window, &editor.borrow());
    let weak = window.as_weak();
    let state = editor.clone();
    window.global::<TweeqBridge>().on_input(move |input| {
        let id = match input.key.as_str() {
            "sample" => ID,
            "angle" => ANGLE,
            "color" => COLOR,
            _ => return false,
        };
        let mut state = state.borrow_mut();
        let modifiers = GestureModifiers {
            fine: input.alt,
            fast: input.shift,
            snap: input.snap,
        };
        let ok = match input.action {
            TweeqAction::BeginText => state.begin_text(id),
            TweeqAction::CommitText => state.commit_text(id, input.text.as_str()),
            TweeqAction::BeginNumber => state.begin_scrub(
                id,
                ScrubStart {
                    x: input.x,
                    width: input.width,
                    modifiers,
                },
            ),
            TweeqAction::BeginRotary => {
                state.begin_rotary(id, [input.x, input.y], [input.width, input.height])
            }
            TweeqAction::Move => state.scrub(id, [input.dx, input.dy], modifiers),
            TweeqAction::Finish => state.commit_scrub(id),
            TweeqAction::BeginColor => state.begin_color(id),
            TweeqAction::PreviewSv => state.preview_sv(id, input.x, input.y),
            TweeqAction::FinishColor => state.commit_color(id),
            TweeqAction::Cancel => {
                state.cancel();
                true
            }
            TweeqAction::Select => {
                state.select(id, input.shift, input.control);
                true
            }
            _ => false,
        };
        if let Some(window) = weak.upgrade() {
            refresh(&window, &state);
        }
        ok
    });

    #[cfg(not(target_arch = "wasm32"))]
    if std::env::args().any(|arg| arg == "--smoke") {
        return smoke(window, editor);
    }
    window.run()
}

#[cfg(not(target_arch = "wasm32"))]
fn smoke(window: Consumer, editor: Rc<RefCell<Editor>>) -> Result<(), slint::PlatformError> {
    use slint::{
        LogicalPosition,
        platform::{Key, PointerEventButton, WindowEvent},
    };
    use tweeq_slint::EditEvent;

    window.show()?;
    let weak = window.as_weak();
    slint::Timer::single_shot(std::time::Duration::from_millis(100), move || {
        let window = weak.upgrade().expect("visible consumer");
        let target = window.window();
        let start = LogicalPosition::new(100.0, 34.0);
        let end = LogicalPosition::new(115.0, 34.0);
        let press = |position| {
            target.dispatch_event(WindowEvent::PointerPressed {
                position,
                button: PointerEventButton::Left,
            })
        };
        let release = |position| {
            target.dispatch_event(WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            })
        };
        target.dispatch_event(WindowEvent::PointerMoved { position: start });
        press(start);
        release(start);
        target.dispatch_event(WindowEvent::KeyPressed {
            text: Key::Return.into(),
        });
        assert_eq!(editor.borrow().number(ID), Some(16_777_217.0));
        let events: Vec<_> = editor.borrow_mut().drain_events().collect();
        assert!(matches!(
            events.as_slice(),
            [EditEvent::Begin { .. }, EditEvent::Commit { .. }]
        ));
        press(start);
        release(start);
        target.dispatch_event(WindowEvent::KeyPressed { text: "4".into() });
        target.dispatch_event(WindowEvent::KeyPressed {
            text: Key::Return.into(),
        });
        assert_eq!(editor.borrow().number(ID), Some(4.0));
        editor.borrow_mut().set_number(ID, 16_777_217.0);
        refresh(&window, &editor.borrow());
        press(start);
        target.dispatch_event(WindowEvent::PointerMoved { position: end });
        release(end);
        assert_eq!(editor.borrow().number(ID), Some(16_777_232.0));
        press(end);
        target.dispatch_event(WindowEvent::PointerMoved { position: start });
        target.dispatch_event(WindowEvent::KeyPressed {
            text: Key::Escape.into(),
        });
        release(start);
        assert_eq!(editor.borrow().number(ID), Some(16_777_232.0));
        let dial = LogicalPosition::new(44.0, 78.0);
        target.dispatch_event(WindowEvent::PointerMoved { position: dial });
        press(dial);
        for (x, y) in [(34.0, 88.0), (24.0, 78.0), (34.0, 68.0), (44.0, 78.0)] {
            target.dispatch_event(WindowEvent::PointerMoved {
                position: LogicalPosition::new(x, y),
            });
        }
        release(dial);
        assert_eq!(editor.borrow().number(ANGLE), Some(360.0));
        let color_start = editor.borrow().value(COLOR);
        let swatch = LogicalPosition::new(32.0, 115.0);
        target.dispatch_event(WindowEvent::PointerMoved { position: swatch });
        press(swatch);
        release(swatch);
        let pad = LogicalPosition::new(120.0, 190.0);
        target.dispatch_event(WindowEvent::PointerMoved { position: pad });
        press(pad);
        release(pad);
        assert_ne!(editor.borrow().value(COLOR), color_start);
        println!(
            "External Slint consumer: numeric editing, a complete turn and color picker passed"
        );
        slint::quit_event_loop().expect("running event loop");
    });
    slint::run_event_loop()
}
