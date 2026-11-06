//! Tweeq Slint gallery.

use std::{cell::RefCell, rc::Rc};

use slint::{Color, ComponentHandle, Model, ModelRc, VecModel};
use tweeq_slint::{
    ColorChannel, EditEvent, Editor, GestureModifiers, NumberConstraints, NumberOptions, ParamId,
    ParamSnapshot, ScrubStart,
};

slint::include_modules!();

const ANGLE: u64 = 4;
const COLOR_A: u64 = 5;
const COLOR_B: u64 = 6;
const ROWS: [(u64, &str, &str); 4] = [
    (1, "Opacity", "Absolute drag, step 0.01, Q snaps to 0.1."),
    (
        2,
        "Exposure",
        "Relative drag, Shift accelerates and Alt makes smaller changes.",
    ),
    (
        3,
        "Precise number",
        "Click and confirm 16777217 without losing precision.",
    ),
    (
        ANGLE,
        "Rotation",
        "Drag around the dial to turn. Q snaps to 45°.",
    ),
];

#[derive(Default)]
struct History {
    capture: Option<Vec<ParamSnapshot>>,
    undo: Vec<Change>,
    redo: Vec<Change>,
}

struct Change {
    before: Vec<ParamSnapshot>,
    after: Vec<ParamSnapshot>,
}

impl History {
    fn record(&mut self, event: EditEvent, editor: &Editor) {
        match event {
            EditEvent::Begin { targets, .. } => self.capture = Some(targets),
            EditEvent::Commit { .. } => {
                if let Some(before) = self.capture.take() {
                    let after: Vec<_> = before
                        .iter()
                        .filter_map(|entry| {
                            editor.value(entry.id).map(|value| ParamSnapshot {
                                id: entry.id,
                                value,
                            })
                        })
                        .collect();
                    if before != after {
                        self.undo.push(Change { before, after });
                        self.redo.clear();
                    }
                }
            }
            EditEvent::Cancel { .. } => self.capture = None,
            EditEvent::Update { .. } => {}
        }
    }

    fn apply(&mut self, editor: &mut Editor, redo: bool) {
        let (source, destination) = if redo {
            (&mut self.redo, &mut self.undo)
        } else {
            (&mut self.undo, &mut self.redo)
        };
        if let Some(change) = source.pop() {
            let values = if redo { &change.after } else { &change.before };
            for snapshot in values {
                editor.set_snapshot(snapshot);
            }
            destination.push(change);
        }
    }
}

struct State {
    editor: Editor,
    history: History,
    last_event: String,
}

impl State {
    fn new() -> Self {
        let mut editor = Editor::default();
        editor.insert_number(
            ParamId::from_u64(1),
            0.4,
            NumberOptions {
                constraints: NumberConstraints {
                    min: Some(0.0),
                    max: Some(1.0),
                    step: Some(0.01),
                    precision: 3,
                    ..NumberConstraints::default()
                },
                snap: Some(0.1),
                default_value: Some(0.4),
            },
        );
        editor.insert_number(
            ParamId::from_u64(2),
            10.0,
            NumberOptions {
                snap: Some(10.0),
                default_value: Some(10.0),
                ..NumberOptions::default()
            },
        );
        editor.insert_number(ParamId::from_u64(3), 16_777_217.0, NumberOptions::default());
        editor.insert_number(
            ParamId::from_u64(ANGLE),
            35.0,
            NumberOptions {
                snap: Some(45.0),
                default_value: Some(35.0),
                ..NumberOptions::default()
            },
        );
        editor.insert_color(ParamId::from_u64(COLOR_A), [0.3, 0.55, 0.92, 1.0]);
        editor.insert_color(ParamId::from_u64(COLOR_B), [0.95, 0.37, 0.32, 0.8]);
        Self {
            editor,
            history: History::default(),
            last_event: "Ready".into(),
        }
    }

    fn input(&mut self, input: &TweeqInput) -> bool {
        let Ok(key) = input.key.parse::<u64>() else {
            return false;
        };
        let id = ParamId::from_u64(key);
        let modifiers = GestureModifiers {
            fine: input.alt,
            fast: input.shift,
            snap: input.snap,
        };
        match input.action {
            TweeqAction::Select => {
                self.editor.select(id, input.shift, input.control);
                true
            }
            TweeqAction::BeginText => self.editor.begin_text(id),
            TweeqAction::CommitText => self.editor.commit_text(id, input.text.as_str()),
            TweeqAction::BeginNumber => self.editor.begin_scrub(
                id,
                ScrubStart {
                    x: input.x,
                    width: input.width,
                    modifiers,
                },
            ),
            TweeqAction::BeginRotary => {
                self.editor
                    .begin_rotary(id, [input.x, input.y], [input.width, input.height])
            }
            TweeqAction::Move => self.editor.scrub(id, [input.dx, input.dy], modifiers),
            TweeqAction::Finish => self.editor.commit_scrub(id),
            TweeqAction::Cancel => {
                self.editor.cancel_from(id);
                true
            }
            TweeqAction::Reset => self.editor.reset_number(id),
            TweeqAction::BeginColor => self.editor.begin_color(id),
            TweeqAction::PreviewChannel => {
                self.editor
                    .preview_channel(id, channel(input.channel), input.x)
            }
            TweeqAction::PreviewSv => self.editor.preview_sv(id, input.x, input.y),
            TweeqAction::FinishColor => self.editor.commit_color(id),
            TweeqAction::SetChannel => {
                self.editor
                    .set_color_channel(id, channel(input.channel), input.text.as_str())
            }
            TweeqAction::SetHex => self.editor.set_color_hex(id, input.text.as_str()),
        }
    }

    fn record_events(&mut self) {
        let events: Vec<_> = self.editor.drain_events().collect();
        for event in events {
            self.last_event = format!("{event:?}");
            self.history.record(event, &self.editor);
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn rows(&self) -> impl Iterator<Item = GalleryRow> + '_ {
        ROWS.into_iter().map(|(id, label, description)| {
            let view = self
                .editor
                .number_view(ParamId::from_u64(id))
                .expect("registered row");
            GalleryRow {
                label: label.into(),
                description: description.into(),
                angle: id == ANGLE,
                rotation: if id == ANGLE {
                    self.editor
                        .number(ParamId::from_u64(id))
                        .unwrap_or_default()
                        .rem_euclid(360.0) as f32
                } else {
                    0.0
                },
                number: TweeqNumberView {
                    key: id.to_string().into(),
                    display: view.display.into(),
                    edit_text: view.edit_text.into(),
                    fraction: view.fraction,
                    bounded: view.bounded,
                    selected: view.selected,
                },
            }
        })
    }

    fn color(&self, id: u64) -> TweeqColorView {
        let view = self
            .editor
            .color_view(ParamId::from_u64(id))
            .expect("registered color");
        let [red, green, blue, alpha] = view.rgba;
        let [hue, saturation, value, _] = view.hsva;
        let [hue_r, hue_g, hue_b, _] = tweeq_slint::hsva_to_rgba([hue, 1.0, 1.0, 1.0]);
        TweeqColorView {
            key: id.to_string().into(),
            rgba: Color::from_argb_f32(alpha, red, green, blue),
            pure_hue: Color::from_argb_f32(1.0, hue_r, hue_g, hue_b),
            hex: view.hex.into(),
            red,
            green,
            blue,
            alpha,
            hue,
            saturation,
            value,
            selected: view.selected,
        }
    }
}

fn channel(value: TweeqChannel) -> ColorChannel {
    match value {
        TweeqChannel::Red => ColorChannel::Red,
        TweeqChannel::Green => ColorChannel::Green,
        TweeqChannel::Blue => ColorChannel::Blue,
        TweeqChannel::Alpha => ColorChannel::Alpha,
        TweeqChannel::Hue => ColorChannel::Hue,
        TweeqChannel::Saturation => ColorChannel::Saturation,
        TweeqChannel::Value => ColorChannel::Value,
    }
}

fn refresh(window: &Gallery, model: &VecModel<GalleryRow>, state: &RefCell<State>) {
    let (rows, colors, log, can_undo, can_redo) = {
        let mut state = state.borrow_mut();
        state.record_events();
        (
            state.rows().collect::<Vec<_>>(),
            [state.color(COLOR_A), state.color(COLOR_B)],
            state.last_event.clone(),
            !state.history.undo.is_empty(),
            !state.history.redo.is_empty(),
        )
    };
    for (index, row) in rows.into_iter().enumerate() {
        if model.row_data(index).as_ref() != Some(&row) {
            model.set_row_data(index, row);
        }
    }
    window.set_event_log(log.into());
    let [first, second] = colors;
    window.set_color_a(first);
    window.set_color_b(second);
    window.set_can_undo(can_undo);
    window.set_can_redo(can_redo);
}

fn connect(window: &Gallery) {
    let state = Rc::new(RefCell::new(State::new()));
    let model = Rc::new(VecModel::from(state.borrow().rows().collect::<Vec<_>>()));
    window.set_rows(ModelRc::from(model.clone()));
    refresh(window, &model, &state);

    let weak = window.as_weak();
    let input_state = state.clone();
    let input_model = model.clone();
    window.global::<TweeqBridge>().on_input(move |input| {
        let accepted = input_state.borrow_mut().input(&input);
        if let Some(window) = weak.upgrade() {
            refresh(&window, &input_model, &input_state);
        }
        accepted
    });

    let history = |redo| {
        let weak = window.as_weak();
        let state = state.clone();
        let model = model.clone();
        move || {
            {
                let mut state = state.borrow_mut();
                state.editor.cancel();
                state.record_events();
                let State {
                    editor, history, ..
                } = &mut *state;
                history.apply(editor, redo);
                state.last_event = if redo { "Redo".into() } else { "Undo".into() };
            }
            if let Some(window) = weak.upgrade() {
                refresh(&window, &model, &state);
            }
        }
    };
    window.on_undo(history(false));
    window.on_redo(history(true));

    let weak = window.as_weak();
    window.window().on_close_requested(move || {
        state.borrow_mut().editor.cancel();
        if let Some(window) = weak.upgrade() {
            refresh(&window, &model, &state);
        }
        slint::CloseRequestResponse::HideWindow
    });
}

/// Launches the gallery on a native window.
///
/// # Errors
///
/// Returns a Slint error if the window or event loop cannot start.
pub fn run() -> Result<(), slint::PlatformError> {
    let window = Gallery::new()?;
    connect(&window);
    window.run()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
/// Starts the gallery on the page's canvas.
///
/// # Errors
///
/// Returns a JavaScript error if the Web window cannot be initialized.
pub fn start() -> Result<(), wasm_bindgen::JsValue> {
    start_web().map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
}

#[cfg(target_arch = "wasm32")]
fn start_web() -> Result<(), slint::PlatformError> {
    // A spawned Winit loop returns normally to Trunk's WASM initializer.
    let backend = i_slint_backend_winit::Backend::builder()
        .with_renderer_name("femtovg")
        .with_spawn_event_loop(true)
        .build()?;
    slint::platform::set_platform(Box::new(backend)).map_err(|error| error.to_string())?;
    let window = Gallery::new()?;
    connect(&window);
    window.show()?;
    slint::run_event_loop()
}
